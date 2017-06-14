#[macro_use]
extern crate serde_derive;
extern crate docopt;
extern crate ksuid;

use std::io;

use ksuid::Ksuid;

const USAGE: &str = "
ksuid

Usage:
    ksuid [--count=<n>]
    ksuid inspect <uids>...

Options:
    -n=<n>, --count=<n>  Number of KSUIDs to generate [default: 1]
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
    for _ in 0..args.flag_count {
        println!("{}", Ksuid::generate().to_base62());
    }
}

fn inspect(args: Args) {
    for uid in args.arg_uids {
        let res = if uid.len() == 40 {
            Ksuid::from_hex(uid.as_ref())
        } else if uid.len() == 27 {
            Ksuid::from_base62(uid.as_ref())
        } else {
            Err(io::Error::new(io::ErrorKind::InvalidData, ""))
        };

        let ksuid = res.expect("Invalid KSUID");

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
        ksuid.time().rfc822(),
        ksuid.timestamp(),
        ksuid.to_hex().chars().skip(8).collect::<String>());
    }
}
