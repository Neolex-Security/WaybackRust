WaybackRust
===

WaybackRust is a tool written in Rust to query the [WaybackMachine](https://archive.org/web/).

Here is the functionalities : 
* Get all urls for a specific domain and get their current HTTP status codes (urls command).
* Get all link in the robots.txt file of every snapshot in the WaybackMachine (robots command).
* Get the source of all archives of a specifics page (unify command).

## Install 

##### Download the statically linked binary from github releases:
* download the static binary : `$ wget https://github.com/Neolex-Security/WaybackRust/releases/download/v0.2.2/waybackrust`
* `$ chmod +x waybackrust`
* `# mv waybackrust /usr/local/bin`
* run waybackrust : `$ waybackrust `

##### from cargo (crates.io):
`cargo install waybackrust`

##### from github:
* Clone this repository `git clone https://github.com/Neolex-Security/WaybackRust`  
* `cargo build --release`
* The executable is in : `./target/release/waybackrust`

## Usage
```
waybackrust 0.2.2
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
    unify     Get the content of all archives for a given url
    urls      Get all urls for a domain

```
###### Urls command :
```
waybackrust-urls 
Get all urls for a domain

USAGE:
    waybackrust urls [FLAGS] [OPTIONS] <domain or file>

FLAGS:
    -h, --help       Prints help information
    -n, --nocheck    Don't check the HTTP status
    -p, --nocolor    Don't colorize HTTP status
        --silent     Disable informations prints
    -s, --subs       Get subdomains too
    -V, --version    Prints version information

OPTIONS:
    -b, --blacklist <extensions to blacklist>    The extensions you want to blacklist (ie: -b png,jpg,txt)
    -d, --delay <delay in milliseconds>          Make a delay between each request (this stops multhreading)
    -o, --output <FILE>                          Name of the file to write the list of urls (default: print on stdout)
    -t, --threads <numbers of threads>           The number of threads you want. (default: 10)
    -w, --whitelist <extensions to whitelist>    The extensions you want to whitelist (ie: -w png,jpg,txt)

ARGS:
    <domain or file>    domain name or file with domains

```

###### Robots command :
```
waybackrust-robots 
Get all disallowed entries from robots.txt

USAGE:
    waybackrust robots [FLAGS] [OPTIONS] <domain or file>

FLAGS:
    -h, --help       Prints help information
        --silent     Disable informations prints
    -V, --version    Prints version information

OPTIONS:
    -o, --output <FILE>                   Name of the file to write the list of uniq paths (default: print on stdout)
    -t, --threads <numbers of threads>    The number of threads you want. (default: 10)

ARGS:
    <domain or file>    domain name or file with domains

```

###### Unify command : 
```
waybackrust-unify 
Get the content of all archives for a given url

USAGE:
    waybackrust unify [FLAGS] [OPTIONS] <url or file>

FLAGS:
    -h, --help       Prints help information
        --silent     Disable informations prints
    -V, --version    Prints version information

OPTIONS:
    -o, --output <FILE>                   Name of the file to write contents of archives (default: print on stdout)
    -t, --threads <numbers of threads>    The number of threads you want. (default: 10)

ARGS:
    <url or file>    url or file with urls

```
## Ideas of new features
If you have idea of improvement and new features in the tool please create an issue or contact me.