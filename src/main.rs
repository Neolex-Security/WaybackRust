extern crate clap;
extern crate regex;
extern crate reqwest;
use clap::{App, Arg, SubCommand};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;

fn main() {
    let app = App::new("waybackrust")
        .version("0.1.2")
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
                    Arg::with_name("nocheck")
                        .short("n")
                        .long("nocheck")
                        .help("Don't check the HTTP status"),
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
                        .takes_value(true),
                ),
        );

    let argsmatches = app.clone().get_matches();

    // get all urls responses codes
    if let Some(argsmatches) = argsmatches.subcommand_matches("urls") {
        let domain = argsmatches.value_of("domain").unwrap();
        let output = Some(argsmatches.value_of("output")).unwrap_or(None);
        let subs = argsmatches.is_present("subs");
        let check = !argsmatches.is_present("nocheck");
        run_urls(domain, subs, check, output);

        return;
    }

    // get all disallow robots
    if let Some(argsmatches) = argsmatches.subcommand_matches("robots") {
        let output = Some(argsmatches.value_of("output")).unwrap_or(None);
        let domain = argsmatches.value_of("domain").unwrap();
        run_robots(domain, output);
        return;
    }

    if let Some(argsmatches) = argsmatches.subcommand_matches("unify") {
        let output = Some(argsmatches.value_of("output")).unwrap_or(None);
        let url = argsmatches.value_of("url").unwrap();
        run_unify(url, output);
        return;
    }
    app.clone().print_help().unwrap();
    println!();
}

fn run_urls(domain: &str, subs: bool, check: bool, output: Option<&str>) {
    let mut pattern = format!("{}/*", domain);
    if subs {
        pattern = format!("*.{}/*", domain);
    }
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
        .collect();
    if check {
        http_status_urls(urls, output);
    } else {
        match output {
            Some(file) => {
                write_string_to_file(urls.join("\n"), file);
                println!("urls saved to {}", file);
            }
            None => println!("{}", urls.join("\n")),
        }
    }
}

fn run_robots(domain: &str, output: Option<&str>) {
    let url = format!("{}/robots.txt", domain);
    let archives = get_archives(url.as_str());
    let all_text = get_all_archives_content(archives);

    let re = Regex::new(r"/.*").unwrap();
    let paths: HashSet<&str> = re
        .find_iter(all_text.as_str())
        .map(|mat| mat.as_str())
        .collect();
    println!("{} uniques paths found:", paths.len());

    let paths_string = paths.into_iter().collect::<Vec<&str>>().join("\n");
    match output {
        Some(file) => {
            write_string_to_file(paths_string, file);
            println!("urls saved to {}", file);
        }
        None => println!("{}", paths_string),
    }
}

fn run_unify(url: &str, output: Option<&str>) {
    let archives = get_archives(url);
    let all_text = get_all_archives_content(archives);
    match output {
        Some(file) => {
            write_string_to_file(all_text, file);
            println!("all archives contents saved to {}", file);
        }
        None => println!("{}", all_text),
    }
}

fn write_string_to_file(string: String, filename: &str) {
    let mut file = File::create(filename).expect("Error creating the file");
    file.write_all(string.as_bytes())
        .expect("Error writing content to the file");
}

fn get_archives(url: &str) -> HashMap<String, String> {
    println!("Looking for archives for {}...", url);
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

fn get_all_archives_content(archives: HashMap<String, String>) -> String {
    println!("Getting {} archives...", archives.len());
    let mut all_text = String::new();
    for (timestamp, url) in archives {
        let timestampurl = format!("https://web.archive.org/web/{}/{}", timestamp, url);
        let response_text = match reqwest::get(timestampurl.as_str()) {
            Ok(mut resp) => resp.text().unwrap_or("".to_string()),
            Err(err) => {
                eprintln!(
                    "Error while parsing response for {} ({})",
                    timestampurl, err
                );
                String::from("")
            }
        };
        all_text.push_str(response_text.as_str());
    }
    all_text
}

fn http_status_urls(urls: Vec<String>, output: Option<&str>) {
    println!("We're checking status of {} urls... ", urls.len());
    let mut ret = String::new();
    for url in urls {
        match reqwest::get(url.as_str()) {
            Ok(response) => {
                let str = format!("{} ({})\n", url, response.status());
                print!("{}", str);
                ret.push_str(str.as_str());
            }
            Err(e) => eprintln!("error geting {} : {}", url, e),
        }
    }

    if let Some(file) = output {
        write_string_to_file(ret, file);
        println!("urls saved to {}", file)
    }
}
