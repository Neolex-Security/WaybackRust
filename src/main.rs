extern crate clap;
extern crate surf;
use ansi_term::Colour;
use clap::{App, AppSettings, Arg, SubCommand};
use futures::future::join_all;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::{thread, time};
use surf::Response;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    #[cfg(target_os = "windows")]
    ansi_term::enable_ansi_support();

    let app = App::new("waybackrust")
        .setting(AppSettings::ArgRequiredElseHelp)
        .version("0.2.0")
        .author("Neolex <hascoet.kevin@neolex-security.fr>")
        .about("Wayback machine tool for bug bounty")
        .subcommand(
            SubCommand::with_name("urls")
                .about("Get all urls for a domain")
                .arg(Arg::with_name("domain")
                    .value_name("domain or file")
                    .help("domain name or file with domains")
                    .required(true)
                    .takes_value(true))
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
                        .help("Make a delay between each request")
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
                ).arg(
                Arg::with_name("blacklist")
                    .short("b")
                    .long("blacklist")
                    .takes_value(true)
                    .value_name("extensions to blacklist")
                    .help("The extensions you want to blacklist (ie: -b png,jpg,txt)")
            ).arg(
                Arg::with_name("whitelist")
                    .short("w")
                    .long("whitelist")
                    .takes_value(true)
                    .value_name("extensions to whitelist")
                    .help("The extensions you want to whitelist (ie: -w png,jpg,txt)")
            ),
        )
        .subcommand(
            SubCommand::with_name("robots")
                .about("Get all disallowed entries from robots.txt")
                .arg(Arg::with_name("domain")
                    .value_name("domain or file")
                    .help("domain name or file with domains")
                    .required(true)
                    .takes_value(true))
                .arg(
                    Arg::with_name("output")
                        .short("o").long("output").value_name("FILE")
                        .help("Name of the file to write the list of uniq paths (default: print on stdout)")
                        .takes_value(true))
                .arg(
                    Arg::with_name("silent")
                        .long("silent")
                        .help("Disable informations prints"),
                ),
        )
        .subcommand(
            SubCommand::with_name("unify")
                .about("Get the content of all archives for a given url")
                .arg(Arg::with_name("url")
                    .value_name("url or file")
                    .help("url or file with urls")
                    .required(true)
                    .takes_value(true))
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
                ),
        );
    let argsmatches = app.clone().get_matches();

    // get all urls responses codes
    if let Some(argsmatches) = argsmatches.subcommand_matches("urls") {
        let domain_or_file = argsmatches.value_of("domain").unwrap();

        let domains = get_domains(domain_or_file);

        let output = Some(argsmatches.value_of("output")).unwrap_or(None);

        let subs = argsmatches.is_present("subs");
        let check = !argsmatches.is_present("nocheck");
        let color = !argsmatches.is_present("nocolor");
        let verbose = !argsmatches.is_present("silent");
        let delay: u64 = match argsmatches.value_of("delay") {
            Some(d) => d.parse().expect("delay must be a number"),
            None => 0,
        };
        if delay > 0 && !check {
            println!(
                "{} delay is useless when --nocheck is used.",
                Colour::RGB(255, 165, 0)
                    .bold()
                    .paint("Warning:")
                    .to_string()
            );
        }
        let blacklist: Vec<String> = match argsmatches.value_of("blacklist") {
            Some(arg) => arg.split(',').map(|ext| [".", ext].concat()).collect(),
            None => Vec::new(),
        };
        let whitelist: Vec<String> = match argsmatches.value_of("whitelist") {
            Some(arg) => arg.split(',').map(|ext| [".", ext].concat()).collect(),
            None => Vec::new(),
        };
        if !blacklist.is_empty() && !whitelist.is_empty() {
            println!(
                "Warning: You set a blacklist and a whitelist. Only the whitelist will be used"
            );
        }
        run_urls(
            domains, subs, check, output, delay, color, verbose, blacklist, whitelist,
        )
        .await;
    }

    // get all disallow robots
    if let Some(argsmatches) = argsmatches.subcommand_matches("robots") {
        let output = Some(argsmatches.value_of("output")).unwrap_or(None);
        let domain_or_file = argsmatches.value_of("domain").unwrap();

        let domains = get_domains(domain_or_file);
        let verbose = !argsmatches.is_present("silent");

        run_robots(domains, output, verbose).await;
    }

    if let Some(argsmatches) = argsmatches.subcommand_matches("unify") {
        let output = Some(argsmatches.value_of("output")).unwrap_or(None);
        let url_or_file = argsmatches.value_of("url").unwrap();

        let urls = get_domains(url_or_file);
        let verbose = !argsmatches.is_present("silent");

        run_unify(urls, output, verbose).await;
    }
}

fn get_domains(domain_or_file: &str) -> Vec<String> {
    if Path::new(domain_or_file).exists() {
        let path = Path::new(domain_or_file);
        let display = path.display();

        // Open the path in read-only mode, returns `io::Result<File>`
        let mut file = match File::open(&path) {
            // The `description` method of `io::Error` returns a string that
            // describes the error
            Err(why) => panic!("couldn't open {}: {}", display, why),
            Ok(file) => file,
        };

        // Read the file contents into a     string, returns `io::Result<usize>`
        let mut s = String::new();
        let content: String = match file.read_to_string(&mut s) {
            Err(why) => panic!("couldn't read {}: {}", display, why),
            Ok(_) => s,
        };

        content.lines().map(String::from).collect()
    } else {
        vec![domain_or_file.to_string()]
    }
}

async fn run_urls(
    domains: Vec<String>,
    subs: bool,
    check: bool,
    output: Option<&str>,
    delay: u64,
    color: bool,
    verbose: bool,
    blacklist: Vec<String>,
    whitelist: Vec<String>,
) {
    let output_string: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let mut join_handles = Vec::new();
    for domain in domains {
        let black = blacklist.clone();
        let white = whitelist.clone();
        let string = Arc::clone(&output_string);
        join_handles.push(tokio::spawn(async move {
            let ret_url = run_url(domain, subs, check, delay, color, verbose, black, white).await;
            string.lock().await.push_str(ret_url.as_str());
        }));
    }
    join_all(join_handles).await;

    if let Some(file) = output {
        write_string_to_file(output_string.lock().await.to_string(), file);
        if verbose {
            println!("urls saved to {}", file)
        };
    }
}

async fn run_url(
    domain: String,
    subs: bool,
    check: bool,
    delay: u64,
    color: bool,
    verbose: bool,
    blacklist: Vec<String>,
    whitelist: Vec<String>,
) -> String {
    let pattern = if subs {
        format!("*.{}/*", domain)
    } else {
        format!("{}/*", domain)
    };
    let url = format!(
        "http://web.archive.org/cdx/search/cdx?url={}&output=text&fl=original&collapse=urlkey",
        pattern
    );
    let urls: Vec<String> = if !whitelist.is_empty() {
        surf::get(url.as_str())
            .await
            .expect("Error GET request")
            .body_string()
            .await
            .expect("Error parsing response")
            .lines()
            .map(|item| item.to_string())
            .filter(|file| whitelist.iter().any(|ext| file.ends_with(ext)))
            .collect()
    } else {
        surf::get(url.as_str())
            .await
            .expect("Error GET request")
            .body_string()
            .await
            .expect("Error parsing response")
            .lines()
            .map(|item| item.to_string())
            .filter(|file| !blacklist.iter().any(|ext| file.ends_with(ext)))
            .collect()
    };
    if check {
        http_status_urls(urls, delay, color, verbose).await
    } else {
        println!("{}", urls.join("\n"));
        urls.join("\n")
    }
}

async fn run_robots(domains: Vec<String>, output: Option<&str>, verbose: bool) {
    let mut output_string = String::new();
    for domain in domains {
        output_string.push_str(run_robot(domain, verbose).await.as_str());
    }
    if let Some(file) = output {
        write_string_to_file(output_string, file);
        if verbose {
            println!("urls saved to {}", file)
        }
    }
}

async fn run_robot(domain: String, verbose: bool) -> String {
    let url = format!("{}/robots.txt", domain);
    let archives = get_archives(url.as_str(), verbose).await;
    get_all_robot_content(archives, verbose).await
}

async fn run_unify(urls: Vec<String>, output: Option<&str>, verbose: bool) {
    let mut output_string = String::new();
    for url in urls {
        let archives = get_archives(url.as_str(), verbose).await;
        let unify_output = get_all_archives_content(archives, verbose).await;
        output_string.push_str(unify_output.as_str());
    }
    if let Some(file) = output {
        write_string_to_file(output_string, file);
        if verbose {
            println!("urls saved to {}", file)
        };
    }
}

fn write_string_to_file(string: String, filename: &str) {
    let mut file = File::create(filename).expect("Error creating the file");
    file.write_all(string.as_bytes())
        .expect("Error writing content to the file");
}

async fn get_archives(url: &str, verbose: bool) -> HashMap<String, String> {
    if verbose {
        println!("Looking for archives for {}...", url)
    };
    let to_fetch= format!("https://web.archive.org/cdx/search/cdx?url={}&output=text&fl=timestamp,original&filter=statuscode:200&collapse=digest", url);
    let lines: Vec<String> = surf::get(to_fetch.as_str())
        .await
        .expect("Error in GET request")
        .body_string()
        .await
        .expect("Error parsing response")
        .lines()
        .map(|x| x.to_owned())
        .collect();
    let mut data = HashMap::new();
    for line in lines {
        match line.split_whitespace().collect::<Vec<&str>>().as_slice() {
            [s1, s2] => {
                data.insert((*s1).to_string(), (*s2).to_string());
            }
            _ => {
                panic!("Invalid Value for archive. line : {}", line);
            }
        }
    }
    data
}

async fn get_all_archives_content(archives: HashMap<String, String>, verbose: bool) -> String {
    if verbose {
        println!("Getting {} archives...", archives.len())
    };

    let mut all_text = String::new();
    for (timestamp, url) in archives {
        let content = get_archive_content(url, timestamp).await;
        if verbose {
            println!("{}", content);
        }
        all_text.push_str(content.as_str());
    }

    all_text.clone()
}

async fn get_all_robot_content(archives: HashMap<String, String>, verbose: bool) -> String {
    if verbose {
        println!("Getting {} archives...", archives.len())
    };

    let all_text: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    for (timestamp, url) in archives {
        let string = Arc::clone(&all_text);
        let content_output = get_archive_content(url, timestamp).await;

        let disallowed_lines: Vec<&str> = content_output
            .lines()
            .filter(|line| line.starts_with("Disallow:"))
            .map(|s| &s[10..])
            .collect();

        for line in disallowed_lines {
            if !string.lock().await.contains(line) {
                string.lock().await.push_str(format!("{}\n", line).as_str());
                if verbose {
                    println!("{}", line);
                }
            }
        }
    }

    all_text.clone().lock().await.to_string()
}

async fn get_archive_content(url: String, timestamp: String) -> String {
    let timestampurl = format!("https://web.archive.org/web/{}/{}", timestamp, url);
    match surf::get(timestampurl.as_str()).await {
        Ok(mut resp) => resp.body_string().await.unwrap_or_else(|_| "".to_string()),
        Err(err) => {
            eprintln!(
                "Error while parsing response for {} ({})",
                timestampurl, err
            );
            String::from("")
        }
    }
}

async fn http_status_urls(urls: Vec<String>, delay: u64, color: bool, verbose: bool) -> String {
    if verbose {
        println!("We're checking status of {} urls... ", urls.len())
    };
    let mut ret: String = String::new();
    for url in urls {
        match surf::get(url.as_str()).await {
            Ok(response) => {
                let str = if color {
                    format!("{} {}\n", url, colorize(&response))
                } else {
                    format!("{} {}\n", url, response.status())
                };
                print!("{}", str);
                ret.push_str(str.as_str());
            }
            Err(e) => eprintln!("error geting {} : {}", url, e),
        }
        if delay > 0 {
            let delay_time = time::Duration::from_millis(delay);
            thread::sleep(delay_time);
        }
    }
    ret
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
