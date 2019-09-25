extern crate clap;
extern crate regex;
extern crate reqwest;
use clap::{App, Arg, SubCommand};
use regex::Regex;
use std::collections::HashSet;

fn main() {
    let app = App::new("waybackrust")
        .version("0.1.0")
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
                .arg(Arg::with_name("nocheck").short("n").long("nocheck").help("Don't check the HTTP status")),
        )
        .subcommand(
            SubCommand::with_name("robots")
                .about("Get all disallowed entries from robots.txt")
                .arg_from_usage("<domain>       'Get disallowed urls from this domain'"),
        );

    let argsmatches = app.clone().get_matches();

    // get all urls responses codes
    if let Some(argsmatches) = argsmatches.subcommand_matches("urls") {
        let domain = argsmatches.value_of("domain").unwrap();
        let subs = argsmatches.is_present("subs");
        let check = !argsmatches.is_present("nocheck");
         wayback_url(domain, subs,check);

        return;
    }

    // get all disallow robots
    if let Some(argsmatches) = argsmatches.subcommand_matches("robots") {
        run_robots(argsmatches.value_of("domain").unwrap());
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
    let urls = match reqwest::get(url.as_str()) {
        Ok(mut response) => match response.text() {
            Ok(r) => r.lines().map(|item| item.to_string()).collect(),
            Err(e) => panic!("Error getting text : {}", e),
        },
        Err(e) => panic!("Error GET request: {}", e),
    };
    if check {
        http_status_urls(urls);
    }else{
        println!("{}",urls.join("\n"));
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
    let url = format!("https://web.archive.org/cdx/search/cdx/?url={}/robots.txt&output=text&fl=timestamp,original&filter=statuscode:200&collapse=digest",domain);

    let text = match reqwest::get(url.as_str()) {
        Ok(mut r) => r.text(),
        Err(e) => panic!("Error in GET request: {}", e),
    };
    let results: Vec<String> = match text {
        Ok(r) => r.lines().map(|item| item.to_string()).collect(),
        Err(e) => panic!("Error : {}", e),
    };
    let number_archives = results.len();
    if number_archives == 0 {
        println!("Found 0 archives of robots.txt for this domain... Quiting.");
        return;
    }
    println!(
        "Found {} robots.txt archives, searching for paths...",
        number_archives
    );

    let mut all_text = String::new();
    for result in results {
        let chunks: Vec<&str> = result.split_whitespace().collect();
        let timestampurl = format!("https://web.archive.org/web/{}/{}", chunks[0], chunks[1]);
        let response = match reqwest::get(timestampurl.as_str()) {
            Ok(mut response) => match response.text() {
                Ok(r) => r,
                Err(e) => panic!("Error GET text : {}", e),
            },
            Err(e) => panic!("Error GET request {} : {}", timestampurl, e),
        };

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
