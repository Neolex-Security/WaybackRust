WaybackRust
===

WaybackRust is a tool written in Rust to query the [WaybackMachine](https://archive.org/web/).

Here is the functionalities : 
* Get all urls for a specific domain and get their current HTTP status codes.
* Get all link in the robots.txt file of every snapshot in the WaybackMachine.

## Install 

* Clone this repository `git clone https://github.com/Neolex-Security/WaybackRust`  
* `cargo build --release`
* The executable is in : `./target/release/waybackrust`

## Usage

```
waybackrust 0.1.0
Neolex <hascoet.kevin@neolex-security.fr>
Wayback machine tool for bug bounty

USAGE:
    waybackrust [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help      Prints this message or the help of the given subcommand(s)
    robots    Get all disallowed entries from robots.txt
    urls      Get all urls for a domain
```
###### Urls command :
```
waybackrust-urls 
Get all urls for a domain

USAGE:
    waybackrust urls [FLAGS] <domain>

FLAGS:
    -h, --help       Prints help information
    -s, --subs       Get subdomains too
    -V, --version    Prints version information

ARGS:
    <domain>    Get urls from this domain
```

###### Robots command :
```
waybackrust-robots 
Get all disallowed entries from robots.txt

USAGE:
    waybackrust robots <domain>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <domain>    Get disallowed urls from this domain
```
