extern crate clap;
use ansi_term::Colour;
use clap::{Arg, Command};
use futures::{stream, StreamExt};
use reqwest::header::{HeaderValue, LOCATION};
use reqwest::{redirect, Response, Url};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::path::{Path,PathBuf};
use std::process;
use std::{io, time};use tokio::time::sleep;

#[tokio::main]
async fn main() {
    #[cfg(target_os = "windows")]
    let _ = ansi_term::enable_ansi_support();

    let argsmatches = Command::new("waybackrust")
        .version("0.2.11")
        .author("Neolex <hascoet.kevin@neolex-security.fr>")
        .about("Wayback machine tool for bug bounty")
        .subcommand(
            Command::new("urls")
                .about("Get all urls for a domain")
                .arg(Arg::new("domain")
                    .value_name("domain.com or file.txt or stdin")
                    .help("domain name or file with domains")
                    .required(true))
                .arg(
                    Arg::new("subs")
                        .short('s')
                        .long("subs")
                        .help("Get subdomains too")
                        .action(clap::ArgAction::SetFalse),
                )
                .arg(
                    Arg::new("silent")
                        .long("silent")
                        .help("Disable informations prints")
                        .action(clap::ArgAction::SetFalse),
                )
                .arg(
                    Arg::new("nocheck")
                        .short('n')
                        .long("nocheck")
                        .help("Don't check the HTTP status")
                        .action(clap::ArgAction::SetFalse),
                )
                .arg(
                    Arg::new("delay")
                        .short('d')
                        .long("delay")
                        .help("Make a delay between each request")
                        .value_name("delay in milliseconds")
                        .value_parser(clap::value_parser!(u64))
                )
                .arg(
                    Arg::new("threads")
                        .short('t')
                        .long("threads")
                        .help("Number of concurrent requests (default: 24)")
                        .value_name("Number of concurrent requests")
                )
                .arg(
                    Arg::new("nocolor")
                        .short('p')
                        .long("nocolor")
                        .help("Don't colorize HTTP status")
                        .action(clap::ArgAction::SetFalse),
                )
                .arg(
                    Arg::new("output_filepath")
                        .short('o')
                        .long("output-file")
                        .value_name("FILE")
                        .value_parser(clap::value_parser!(PathBuf))
                        .help(
                            "Name of the file to write the list of urls (default: print on stdout)",
                        )
                ).arg(
                Arg::new("blacklist")
                    .short('b')
                    .long("blacklist")
                    .value_name("extensions to blacklist")
                    .help("The extensions you want to blacklist (ie: -b png,jpg,txt)")
            ).arg(
                Arg::new("whitelist")
                    .short('w')
                    .long("whitelist")
                    .value_name("extensions to whitelist")
                    .help("The extensions you want to whitelist (ie: -w png,jpg,txt)")
            ).arg(
                Arg::new("blacklist code")
                    .short('z')
                    .long("blacklist-code")
                    .value_name("codes to blacklist")
                    .help("The status codes you want to blacklist (ie: --blacklist-code 404,403,500)")
            ).arg(
                Arg::new("whitelist code")
                    .short('c')
                    .long("whitelist-code")
                    .value_name("codes to whitelist")
                    .help("The status codes you want to blacklist (ie: --whitelist-code 404,403,500)")
            )
        )
        .subcommand(
            Command::new("robots")
                .about("Get all disallowed entries from robots.txt")
                .arg(Arg::new("domain")
                    .value_name("domain.com or file.txt or stdin")
                    .help("domain name or file with domains")
                    .required(true))
                .arg(
                    Arg::new("output_filepath")
                        .short('o').long("output-file").value_name("FILE")
                        .help("Name of the file to write the list of uniq paths (default: print on stdout)"))
                .arg(
                    Arg::new("silent")
                        .long("silent")
                        .help("Disable informations prints")
                        .action(clap::ArgAction::SetFalse),
                ),
        )
        .subcommand(
            Command::new("unify")
                .about("Get the content of all archives for a given url")
                .arg(Arg::new("url")
                    .value_name("url or file")
                    .help("url or file with urls")
                    .required(true))
                .arg(
                    Arg::new("output_filepath")
                        .short('o')
                        .long("output-file")
                        .value_name("FILE")
                        .help("Name of the file to write contents of archives (default: print on stdout)"))
                .arg(
                    Arg::new("silent")
                        .long("silent")
                        .help("Disable informations prints"),
                ),
        ).get_matches();
    // get all urls responses codes
    if let Some(argsmatches) = argsmatches.subcommand_matches("urls") {
        let domain_or_file = argsmatches.get_one::<String>("domain").unwrap();

        let domains = get_domains(domain_or_file);

        let output_filepath
         = argsmatches.get_one::<PathBuf>("output_filepath");

        let subs = argsmatches.get_flag("subs");
        let check = argsmatches.get_flag("nocheck");

        let color = argsmatches.get_flag("nocolor");
        let verbose = argsmatches.get_flag("silent");
        let delay = argsmatches.get_one::<u64>("delay").unwrap_or(&0);
        let workers = match argsmatches.get_one::<usize>("threads") {
            Some(d) => {
                if delay > &0 {
                    println!(
                        "{} you set a delay and a number of threads, there  will only be one thread.",
                        Colour::RGB(255, 165, 0)
                            .bold()
                            .paint("Warning:")
                            .to_string()
                    );
                    &0
                } else {
                    d
                }
            }
            None => &24,
        };

        if delay > &0 && !check {
            println!(
                "{} delay is useless when --nocheck is used.",
                Colour::RGB(255, 165, 0)
                    .bold()
                    .paint("Warning:")
                    .to_string()
            );
        }
        let blacklist: Vec<String> = match argsmatches.get_one::<String>("blacklist") {
            Some(arg) => arg.split(',').map(|ext| [".", ext].concat()).collect(),
            None => Vec::new(),
        };
        let blacklist_code: Vec<u16> = match argsmatches.get_one::<String>("blacklist code" ) {
            Some(arg) => arg.split(',').map(|code| code.parse::<u16>().unwrap()).collect::<Vec<u16>>(),
            None => Vec::new(),
        };
        let whitelist_code: Vec<u16> = match argsmatches.get_one::<String>("whitelist code" ) {
            Some(arg) => arg.split(',').map(|code| code.parse::<u16>().unwrap()).collect::<Vec<u16>>(),
            None => Vec::new(),
        };
        let whitelist: Vec<String> = match argsmatches.get_one::<String>("whitelist") {
            Some(arg) => arg.split(',').map(|ext| [".", ext].concat()).collect(),
            None => Vec::new(),
        };
        if !blacklist.is_empty() && !whitelist.is_empty() {
            println!(
                "{} You set a blacklist and a whitelist. Only the whitelist will be used.",
                Colour::RGB(255, 165, 0)
                    .bold()
                    .paint("Warning:")
                    .to_string()
            );
        }
        run_urls(
            domains, subs, check, output_filepath
            , *delay, color, verbose, blacklist, whitelist, *workers,blacklist_code,whitelist_code
        )
        .await;
    }

    // get all disallow robots
    if let Some(argsmatches) = argsmatches.subcommand_matches("robots") {
        let output_filepath = argsmatches.get_one::<PathBuf>("output_filepath");
        let domain_or_file = argsmatches.get_one::<String>("domain").unwrap();
        let domains = get_domains(domain_or_file);
        let verbose = argsmatches.get_flag("silent");

        run_robots(domains, output_filepath
            , verbose).await;
    }

    if let Some(argsmatches) = argsmatches.subcommand_matches("unify") {
        let output_filepath
         = argsmatches.get_one::<PathBuf>("output_filepath
        ");
        let url_or_file = argsmatches.get_one::<String>("url").unwrap();

        let urls = get_domains(url_or_file);
        let verbose = argsmatches.get_flag("silent");

        run_unify(urls, output_filepath
            , verbose).await;
    }
}

fn get_domains(domain_or_file: &String) -> Vec<String> {
    if domain_or_file.ne("stdin") {
        if Path::new(domain_or_file).is_file() {
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
    } else {
        let mut s = String::new();
        let content: String = match io::stdin().read_to_string(&mut s) {
            Err(why) => panic!("couldn't read stdin {}", why),
            Ok(_) => s,
        };

        content.lines().map(String::from).collect()
    }
}

async fn run_urls(
    domains: Vec<String>,
    subs: bool,
    check: bool,
    output_filepath
    : Option<&PathBuf>,
    delay: u64,
    color: bool,
    verbose: bool,
    blacklist: Vec<String>,
    whitelist: Vec<String>,
    workers: usize,
    blacklist_code: Vec<u16>,
    whitelist_code: Vec<u16>
) {
    let mut join_handles = Vec::with_capacity(domains.len());
    for domain in domains {
        let black = blacklist.clone();
        let white = whitelist.clone();
        let blacklist_code = blacklist_code.clone();
        let whitelist_code = whitelist_code.clone();
        join_handles.push(tokio::spawn(async move {
            run_url(
                domain, subs, check, delay, color, verbose, black, white, workers,blacklist_code,whitelist_code
            )
            .await
        }));
    }

    let mut output_string = String::new();
    for handle in join_handles {
        let ret_url = handle.await.expect("fail");
        output_string.push_str(ret_url.as_str());
    }

    if let Some(path) = output_filepath
     {
        write_string_to_file(output_string, path);
        if verbose {
            println!("urls saved to {}", path.display())
        };
    }
}
fn get_path(url: &str) -> String {
    match Url::parse(&url) {
        Ok(parsed) => parsed.path().to_string(),
        Err(_) => "".to_string(),
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
    workers: usize,
    blacklist_code: Vec<u16>,
    whitelist_code: Vec<u16>
) -> String {
    let pattern = if subs {
        format!("*.{}%2F*", domain)
    } else {
        format!("{}%2F*", domain)
    };
    let url = format!(
        "http://web.archive.org/cdx/search/cdx?url={}&output_filepath
        =text&fl=original&collapse=urlkey",
        pattern
    );

    let response = match reqwest::get(url.as_str()).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(-1)
        }
    };
    let response_text = match response.text().await {
        Ok(txt) => txt,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(-1)
        }
    };

    let lines = response_text.lines().map(|item| item.to_string());

    let urls: Vec<String> = if !whitelist.is_empty() {
        lines
            .filter(|url| whitelist.iter().any(|ext| get_path(url).ends_with(ext)))
            .collect()
    } else {
        lines
            .filter(|url| !blacklist.iter().any(|ext| get_path(url).ends_with(ext)))
            .collect()
    };
    if check {
        if delay > 0 {
            http_status_urls_delay(urls, delay, color, verbose, blacklist_code,whitelist_code).await
        } else {
            http_status_urls_no_delay(urls, color, verbose, workers, blacklist_code,whitelist_code).await
        }
    } else {
        println!("{}", urls.join("\n"));
        urls.join("\n")
    }
}

async fn run_robots(domains: Vec<String>, output_filepath
    : Option<&PathBuf>, verbose: bool) {
    let mut output_string = String::new();
    for domain in domains {
        output_string.push_str(run_robot(domain, verbose).await.as_str());
    }
    if let Some(path) = output_filepath
     {
        write_string_to_file(output_string, path);
        if verbose {
            println!("urls saved to {}", path.display())
        }
    }
}

async fn run_robot(domain: String, verbose: bool) -> String {
    let url = format!("{}/robots.txt", domain);
    let archives = get_archives(url.as_str(), verbose).await;
    get_all_robot_content(archives, verbose).await
}

async fn run_unify(urls: Vec<String>, output_filepath
    : Option<&PathBuf>, verbose: bool) {
    let mut output_string = String::new();
    for url in urls {
        let archives = get_archives(url.as_str(), verbose).await;
        let unify_output = get_all_archives_content(archives, verbose).await;
        output_string.push_str(unify_output.as_str());
    }
    if let Some(path) = output_filepath
     {
        write_string_to_file(output_string, path);
        if verbose {
            println!("urls saved to {}", path.display())
        };
    }
}

fn write_string_to_file(string: String, filename: &Path) {
    let mut file = File::create(filename).expect("Error creating the file");
    file.write_all(string.as_bytes())
        .expect("Error writing content to the file");
}

async fn get_archives(url: &str, verbose: bool) -> HashMap<String, String> {
    if verbose {
        println!("Looking for archives for {}...", url)
    };
    let to_fetch= format!("https://web.archive.org/cdx/search/cdx?url={}&output_filepath=text&fl=timestamp,original&filter=statuscode:200&collapse=digest", url);
    let lines: Vec<String> = reqwest::get(to_fetch.as_str())
        .await
        .expect("Error in GET request")
        .text()
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

    let mut output_string = String::new();

    for (timestamp, url) in archives {
        let archive_content = get_archive_content(url, timestamp).await;

        let disallowed_lines: Vec<String> = archive_content
            .lines()
            .filter(|line| line.contains("low:"))
            .map(|s| s.replace("Disallow:", "").replace("Allow:", ""))
            .collect();

        for line in disallowed_lines {
            if !output_string.contains(&line) {
                output_string.push_str(format!("{}\n", line).as_str());
                if verbose {
                    println!("{}", line.trim());
                }
            }
        }
    }
    output_string
}

async fn get_archive_content(url: String, timestamp: String) -> String {
    let timestampurl = format!("https://web.archive.org/web/{}/{}", timestamp, url);
    match reqwest::get(timestampurl.as_str()).await {
        Ok(resp) => resp.text().await.unwrap_or_else(|_| "".to_string()),
        Err(err) => {
            eprintln!(
                "Error while parsing response for {} ({})",
                timestampurl, err
            );
            String::from("")
        }
    }
}

async fn http_status_urls_delay(
    urls: Vec<String>,
    delay: u64,
    color: bool,
    verbose: bool,
    blacklist_code: Vec<u16>,
    whitelist_code: Vec<u16>
) -> String {
    if verbose {
        println!("We're checking status of {} urls... ", urls.len())
    };
    let mut ret: String = String::new();

    let client = reqwest::ClientBuilder::new()
        .redirect(redirect::Policy::none())
        .build()
        .unwrap();

    for url in urls {
        match client.get(&url).send().await {
            Ok(response) => {
                if delay > 0 {
                    let delay_time = time::Duration::from_millis(delay);
                    sleep(delay_time).await;
                }
                if whitelist_code.is_empty() || (&whitelist_code).into_iter().any(|code| *code == response.status().as_u16()) {
                    if !(&blacklist_code).into_iter().any(|code| *code == response.status().as_u16()) {
                        let str_output = if color {
                            format!("{} {}\n", &url, colorize(&response))
                        } else if response.status().is_redirection() {
                            format!(
                                "{} {} to {}\n",
                                &url,
                                &response.status(),
                                &response
                                    .headers()
                                    .get(LOCATION)
                                    .unwrap_or(&HeaderValue::from_str("").unwrap())
                                    .to_str()
                                    .unwrap()
                            )
                        } else {
                            format!("{} {}\n", &url, &response.status())
                        };

                        print!("{}", str_output);
                        ret.push_str(&str_output);
                    }
                }
            }
            Err(_e) => {
            }
        }
    }

    ret
}

async fn http_status_urls_no_delay(
    urls: Vec<String>,
    color: bool,
    verbose: bool,
    workers: usize,
    blacklist_code: Vec<u16>,
    whitelist_code: Vec<u16>
) -> String {
    if verbose {
        println!("We're checking status of {} urls... ", urls.len())
    };
    let client = reqwest::ClientBuilder::new()
        .redirect(redirect::Policy::none())
        .build()
        .unwrap();
    let mut bodies = stream::iter(urls)
        .map(|url| async { (client.get(&url).send().await, url) })
        .buffer_unordered(workers);
    let mut ret: String = String::new();

    while let Some(b) = bodies.next().await {
        match b.0 {
            Ok(response) => {
                if whitelist_code.is_empty() || (&whitelist_code).into_iter().any(|code| *code == response.status().as_u16()) {
                    if !(&blacklist_code).into_iter().any(|code| *code == response.status().as_u16()) {
                        let str_output = if color {
                            format!("{} {}\n", &b.1, colorize(&response))
                        } else if response.status().is_redirection() {
                            format!(
                                "{} {} to {}\n",
                                &b.1,
                                &response.status(),
                                &response
                                    .headers()
                                    .get(LOCATION)
                                    .unwrap_or(&HeaderValue::from_str("").unwrap())
                                    .to_str()
                                    .unwrap()
                            )
                        } else {
                            format!("{} {}\n", &b.1, &response.status())
                        };
                        print!("{}", str_output);
                        ret.push_str(&str_output);
                    }
                }
            }
            Err(e) => {
                eprintln!("error geting : {}", e);
            }
        }
    }

    ret
}

fn colorize(response: &Response) -> String {
    let status = response.status().to_string();

    let status_col = match status.as_str() {
        "200 OK" => Colour::Green.bold().paint(&status).to_string(),
        "404 Not Found" => Colour::Red.bold().paint(&status).to_string(),
        "403 Forbidden" => Colour::Purple.bold().paint(&status).to_string(),
        _ => Colour::RGB(255, 165, 0).bold().paint(&status).to_string(),
    };
    if response.status().is_redirection() {
        format!(
            "{} to {}",
            status_col,
            &response
                .headers()
                .get(LOCATION)
                .unwrap_or(&HeaderValue::from_str("").unwrap())
                .to_str()
                .unwrap_or("")
        )
    } else {
        status_col
    }
}
