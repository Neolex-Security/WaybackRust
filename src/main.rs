extern crate clap;
extern crate regex;
extern crate reqwest;
extern crate threadpool;
use ansi_term::Colour;
use clap::{App, AppSettings, Arg, SubCommand};
use regex::Regex;
use reqwest::Response;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::{thread, time};

fn main() {
    #[cfg(target_os = "windows")]
    ansi_term::enable_ansi_support();

    let app = App::new("waybackrust")
        .setting(AppSettings::ArgRequiredElseHelp)
        .version("0.1.9")
        .author("Neolex <hascoet.kevin@neolex-security.fr>")
        .about("Wayback machine tool for bug bounty")
        .subcommand(
            SubCommand::with_name("urls")
                .about("Get all urls for a domain")
                .arg_from_usage("<domain>       'Get urls from this domain'")
                .arg(
                    Arg::with_name("subs")
                        .short("s")
                        .long("subs")
                        .help("Get subdomains too"),
                )
                .arg(
                    Arg::with_name("silent")
                        .long("silent")
                        .help("Disable informations prints"),
                )
                .arg(
                    Arg::with_name("nocheck")
                        .short("n")
                        .long("nocheck")
                        .help("Don't check the HTTP status"),
                )
                .arg(
                    Arg::with_name("delay")
                        .short("d")
                        .long("delay")
                        .help("Make a delay between each request (this stops multhreading)")
                        .value_name("delay in milliseconds")
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("nocolor")
                        .short("p")
                        .long("nocolor")
                        .help("Don't colorize HTTP status"),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .value_name("FILE")
                        .help(
                            "Name of the file to write the list of urls (default: print on stdout)",
                        )
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("threads")
                        .short("t")
                        .long("threads")
                        .takes_value(true)
                        .value_name("numbers of threads")
                        .help("The number of threads you want. (default: 10)")
                ).arg(
                    Arg::with_name("blacklist")
                        .short("b")
                        .long("blacklist")
                        .takes_value(true)
                        .value_name("extensions to blacklist")
                        .help("The extensions you want to blacklist (ie: -b png,jpg,txt)")
            ),
        )
        .subcommand(
            SubCommand::with_name("robots")
                .about("Get all disallowed entries from robots.txt")
                .arg_from_usage("<domain>       'Get disallowed urls from this domain'")
                .arg(
                    Arg::with_name("output")
                     .short("o").long("output").value_name("FILE")
                     .help("Name of the file to write the list of uniq paths (default: print on stdout)")
                      .takes_value(true))
                .arg(
                    Arg::with_name("silent")
                        .long("silent")
                        .help("Disable informations prints"),
                )
                .arg(
                    Arg::with_name("threads")
                        .short("t")
                        .long("threads")
                        .takes_value(true)
                        .value_name("numbers of threads")
                        .help("The number of threads you want. (default: 10)")
                ),
        )
        .subcommand(
            SubCommand::with_name("unify")
                .about("Get the content of all archives for a given url")
                .arg_from_usage("<url>       'The url you want to unify'")
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .value_name("FILE")
                        .help("Name of the file to write contents of archives (default: print on stdout)")
                        .takes_value(true))
                .arg(
                    Arg::with_name("silent")
                        .long("silent")
                        .help("Disable informations prints"),
                )
                .arg(
                    Arg::with_name("threads")
                        .short("t")
                        .long("threads")
                        .takes_value(true)
                        .value_name("numbers of threads")
                        .help("The number of threads you want. (default: 10)")
                ),
        );
    let argsmatches = app.clone().get_matches();

    // get all urls responses codes
    if let Some(argsmatches) = argsmatches.subcommand_matches("urls") {
        let domain = argsmatches.value_of("domain").unwrap();
        let output = Some(argsmatches.value_of("output")).unwrap_or(None);
        let mut threads: usize = match argsmatches.value_of("threads") {
            Some(o) => o.parse().expect("threads must be a number"),
            None => 10,
        };

        let subs = argsmatches.is_present("subs");
        let check = !argsmatches.is_present("nocheck");
        let color = !argsmatches.is_present("nocolor");
        let verbose = !argsmatches.is_present("silent");
        let delay: u64 = match argsmatches.value_of("delay") {
            Some(d) => d.parse().expect("delay must be a number"),
            None => 0,
        };
        if delay > 0 {
            if !check {
                println!(
                    "{} delay is useless when --nocheck is used.",
                    Colour::RGB(255, 165, 0)
                        .bold()
                        .paint("Warning:")
                        .to_string()
                );
            }
            threads = 1;
        }
        let blacklist: Vec<String> = match argsmatches.value_of("blacklist") {
            Some(arg) => arg.split(',').map(|ext| [".", ext].concat()).collect(),
            None => Vec::new(),
        };
        run_urls(
            domain, subs, check, output, threads, delay, color, verbose, blacklist,
        );

        return;
    }

    // get all disallow robots
    if let Some(argsmatches) = argsmatches.subcommand_matches("robots") {
        let output = Some(argsmatches.value_of("output")).unwrap_or(None);
        let domain = argsmatches.value_of("domain").unwrap();
        let threads: usize = match argsmatches.value_of("threads") {
            Some(o) => o.parse().expect("threads must be a number"),
            None => 10,
        };
        let verbose = !argsmatches.is_present("silent");

        run_robots(domain, output, threads, verbose);
        return;
    }

    if let Some(argsmatches) = argsmatches.subcommand_matches("unify") {
        let output = Some(argsmatches.value_of("output")).unwrap_or(None);
        let url = argsmatches.value_of("url").unwrap();
        let threads: usize = match argsmatches.value_of("threads") {
            Some(o) => o.parse().expect("threads must be a number"),
            None => 10,
        };
        let verbose = !argsmatches.is_present("silent");

        run_unify(url, output, threads, verbose);
        return;
    }
}

fn run_urls(
    domain: &str,
    subs: bool,
    check: bool,
    output: Option<&str>,
    threads: usize,
    delay: u64,
    color: bool,
    verbose: bool,
    blacklist: Vec<String>,
) {
    let pattern = if subs {
        format!("*.{}/*", domain)
    } else {
        format!("{}/*", domain)
    };
    let url = format!(
        "http://web.archive.org/cdx/search/cdx?url={}&output=text&fl=original&collapse=urlkey",
        pattern
    );
    let urls: Vec<String> = reqwest::get(url.as_str())
        .expect("Error GET request")
        .text()
        .expect("Error parsing response")
        .lines()
        .map(|item| item.to_string())
        .filter(|file| !blacklist.iter().any(|ext| file.ends_with(ext)))
        .collect();
    if check {
        http_status_urls(urls, output, threads, delay, color, verbose);
    } else {
        match output {
            Some(file) => {
                write_string_to_file(urls.join("\n"), file);
                if verbose {
                    println!("urls saved to {}", file)
                };
            }
            None => println!("{}", urls.join("\n")),
        }
    }
}

fn run_robots(domain: &str, output: Option<&str>, threads: usize, verbose: bool) {
    let url = format!("{}/robots.txt", domain);
    let archives = get_archives(url.as_str(), verbose);
    let all_text = get_all_archives_content(archives, threads, verbose);

    let re = Regex::new(r"/.*").unwrap();
    let paths: HashSet<&str> = re
        .find_iter(all_text.as_str())
        .map(|mat| mat.as_str())
        .collect();
    if verbose {
        println!("{} uniques paths found:", paths.len())
    };

    let paths_string = paths.into_iter().collect::<Vec<&str>>().join("\n");
    match output {
        Some(file) => {
            write_string_to_file(paths_string, file);
            if verbose {
                println!("urls saved to {}", file)
            };
        }
        None => println!("{}", paths_string),
    }
}

fn run_unify(url: &str, output: Option<&str>, threads: usize, verbose: bool) {
    let archives = get_archives(url, verbose);
    let all_text = get_all_archives_content(archives, threads, verbose);
    match output {
        Some(file) => {
            write_string_to_file(all_text, file);
            if verbose {
                println!("all archives contents saved to {}", file)
            };
        }
        None => println!("{}", all_text),
    }
}

fn write_string_to_file(string: String, filename: &str) {
    let mut file = File::create(filename).expect("Error creating the file");
    file.write_all(string.as_bytes())
        .expect("Error writing content to the file");
}

fn get_archives(url: &str, verbose: bool) -> HashMap<String, String> {
    if verbose {
        println!("Looking for archives for {}...", url)
    };
    let to_fetch= format!("https://web.archive.org/cdx/search/cdx?url={}&output=text&fl=timestamp,original&filter=statuscode:200&collapse=digest", url);
    let lines: Vec<String> = reqwest::get(to_fetch.as_str())
        .expect("Error in GET request")
        .text()
        .expect("Error parsing response")
        .lines()
        .map(|x| x.to_owned())
        .collect();
    let mut data = HashMap::new();
    for line in lines {
        match line.split_whitespace().collect::<Vec<&str>>().as_slice() {
            [s1, s2] => {
                data.insert(s1.to_string(), s2.to_string());
            }
            _ => {
                panic!("Invalid Value for archive. line : {}", line);
            }
        }
    }
    data
}

fn get_all_archives_content(
    archives: HashMap<String, String>,
    threads: usize,
    verbose: bool,
) -> String {
    if verbose {
        println!("Getting {} archives...", archives.len())
    };
    let pool = threadpool::Builder::new().num_threads(threads).build();

    let all_text = Arc::new(Mutex::new(String::new()));

    for (timestamp, url) in archives {
        let all_text = Arc::clone(&all_text);
        pool.execute(move || {
            let timestampurl = format!("https://web.archive.org/web/{}/{}", timestamp, url);
            let response_text = match reqwest::get(timestampurl.as_str()) {
                Ok(mut resp) => resp.text().unwrap_or_else(|_| "".to_string()),
                Err(err) => {
                    eprintln!(
                        "Error while parsing response for {} ({})",
                        timestampurl, err
                    );
                    String::from("")
                }
            };
            all_text
                .lock()
                .expect("Error locking the mutex")
                .push_str(response_text.as_str());
        });
    }
    pool.join();
    all_text
        .clone()
        .lock()
        .expect("Error locking the mutex")
        .to_string()
}

fn http_status_urls(
    urls: Vec<String>,
    output: Option<&str>,
    threads: usize,
    delay: u64,
    color: bool,
    verbose: bool,
) {
    if verbose {
        println!("We're checking status of {} urls... ", urls.len())
    };

    let pool = threadpool::Builder::new().num_threads(threads).build();

    let ret = Arc::new(Mutex::new(String::new()));
    for url in urls {
        let ret = Arc::clone(&ret);
        pool.execute(move || {
            thread::sleep(time::Duration::from_millis(delay));
            match reqwest::get(url.as_str()) {
                Ok(response) => {
                    let str = if color {
                        format!("{} {}\n", url, colorize(&response))
                    } else {
                        format!("{} {}\n", url, response.status())
                    };
                    print!("{}", str);
                    ret.lock()
                        .expect("Error locking the mutex")
                        .push_str(str.as_str());
                }
                Err(e) => eprintln!("error geting {} : {}", url, e),
            }
        });
    }
    pool.join();
    if let Some(file) = output {
        write_string_to_file(
            ret.lock().expect("Error locking the mutex").to_string(),
            file,
        );
        if verbose {
            println!("The urls are saved to file {}", file)
        }
    }
}

fn colorize(response: &Response) -> String {
    let status = response.status().to_string();
    match status.as_ref() {
        "200 OK" => Colour::Green.bold().paint(status).to_string(),
        "404 Not Found" => Colour::Red.bold().paint(status).to_string(),
        "403 Forbidden" => Colour::Purple.bold().paint(status).to_string(),
        _ => Colour::RGB(255, 165, 0).bold().paint(status).to_string(),
    }
}
