#[macro_use]
extern crate serde_derive;

extern crate docopt;
extern crate ksuid;
extern crate time;
extern crate rand;

use std::io::{self, Write};
use std::process::exit;

use ksuid::Ksuid;
use rand::Rng;

const USAGE: &str = "
ksuid

Usage:
    ksuid [-n <n> | --count=<n>]
    ksuid inspect <uids>...
    ksuid -h | --help

Options:
    -h, --help           Display this help and exit
    -n <n>, --count=<n>  Number of KSUIDs to generate [default: 1]
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_count: usize,
    arg_uids: Vec<String>,
    cmd_inspect: bool,
}

fn main() {
    let args: Args = docopt::Docopt::new(USAGE)
                                     .and_then(|d| d.deserialize())
                                     .unwrap_or_else(|e| e.exit());

    if args.cmd_inspect {
        inspect(args)
    } else {
        generate(args)
    }
}

fn generate(args: Args) {
    let out = io::stdout();
    let mut locked = out.lock();

    let mut rng = rand::thread_rng();

    for _ in 0..args.flag_count {
        writeln!(&mut locked, "{}", rng.gen::<Ksuid>().to_base62()).unwrap();
    }
}

fn inspect(args: Args) {
    for uid in args.arg_uids {
        let res = if uid.len() == 40 {
            Ksuid::from_hex(uid.as_ref())
        } else if uid.len() == 27 {
            Ksuid::from_base62(uid.as_ref())
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, "KSUID must be either 27 characters (Base62) or 40 characters (Hex) in length"))
        };

        let ksuid = match res {
            Ok(id) => id,
            Err(e) => {
                let _ = writeln!(io::stderr(), "Invalid KSUID: {}", e);
                exit(e.raw_os_error().unwrap_or(2));
            }
        };

        println!("
REPRESENTATION:

  String: {}
     Raw: {}

COMPONENTS:

       Time: {}
  Timestamp: {}
    Payload: {}
"       ,
        ksuid.to_base62(),
        ksuid.to_hex(),
        time::at(ksuid.time()).rfc822(),
        ksuid.timestamp(),
        ksuid.to_hex().chars().skip(8).collect::<String>());
    }
}
