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
        .version("0.1.1")
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
                ),
        )
        .subcommand(
            SubCommand::with_name("robots")
                .about("Get all disallowed entries from robots.txt")
                .arg_from_usage("<domain>       'Get disallowed urls from this domain'"),
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
                        .help("name of the file to write contents of archives")
                        .takes_value(true),
                )
        );

    let argsmatches = app.clone().get_matches();

    // get all urls responses codes
    if let Some(argsmatches) = argsmatches.subcommand_matches("urls") {
        let domain = argsmatches.value_of("domain").unwrap();
        let subs = argsmatches.is_present("subs");
        let check = !argsmatches.is_present("nocheck");
        wayback_url(domain, subs, check);

        return;
    }

    // get all disallow robots
    if let Some(argsmatches) = argsmatches.subcommand_matches("robots") {
        run_robots(argsmatches.value_of("domain").unwrap());
        return;
    }

    if let Some(argsmatches) = argsmatches.subcommand_matches("unify") {
        let output = argsmatches.value_of("output").unwrap_or("output.txt");
        run_unify(argsmatches.value_of("url").unwrap(), &output.to_string());
        return;
    }
    app.clone().print_help().unwrap();
    println!();
}

fn wayback_url(domain: &str, subs: bool, check: bool) {
    let mut pattern = format!("{}/*", domain);
    if subs {
        pattern = format!("*.{}/*", domain);
    }
    let url = format!(
        "http://web.archive.org/cdx/search/cdx?url={}&output=text&fl=original&collapse=urlkey",
        pattern
    );
    let urls = reqwest::get(url.as_str())
        .expect("Error GET request")
        .text()
        .expect("Error parsing response")
        .lines()
        .map(|item| item.to_string())
        .collect();
    if check {
        http_status_urls(urls);
    } else {
        println!("{}", urls.join("\n"));
    }
}

fn http_status_urls(urls: Vec<String>) {
    for url in urls {
        match reqwest::get(url.as_str()) {
            Ok(response) => println!("{} ({})", url, response.status()),
            Err(e) => println!("error geting {} : {}", url, e),
        }
    }
}

fn run_robots(domain: &str) {
    let url = format!("{}/robots.txt", domain);
    let archives = get_archives(url.as_str());

    let mut all_text = String::new();
    for (timestamp, url) in archives {
        let timestampurl = format!("https://web.archive.org/web/{}/{}", timestamp, url);
        let response = reqwest::get(timestampurl.as_str())
            .expect("Error GET request")
            .text()
            .expect("Error parsing request");

        if response.contains("Disallow:") {
            all_text.push_str(response.as_str())
        }
    }
    let re = Regex::new(r"/.*").unwrap();
    let paths: HashSet<&str> = re
        .find_iter(all_text.as_str())
        .map(|mat| mat.as_str())
        .collect();
    println!("{} uniques paths found:", paths.len());
    for path in paths {
        println!("{}", path);
    }
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
    println!("Found {} archives...", data.len());
    data
}

fn run_unify(url: &str, output: &String) {
    let archives = get_archives(url);

    let mut all_text = String::new();
    for (timestamp, url) in archives {
        let timestampurl = format!("https://web.archive.org/web/{}/{}", timestamp, url);
        let response = reqwest::get(timestampurl.as_str())
            .expect("Error GET request")
            .text()
            .expect("Error parsing request");

        all_text.push_str(response.as_str());
    }
    let mut file = File::create(output).expect("Error creating the file");
    file.write_all(all_text.as_bytes())
        .expect("Error writing content to the file");
    println!(
        "Content of achivres that points to {} saved in {}",url, output
    );
}
