use std::{env, fs, io::{self, Write}, thread, time::Duration};

fn main() {
    let args: Vec<_> = env::args().collect();
    match args[1].as_str() {
        "match" => eprintln!("match rules. test"),
        "fail" => std::process::exit(1),
        "flood" => { io::stderr().write_all(&vec![b'x'; 128 * 1024]).unwrap(); }
        "idle" => {
            fs::write(&args[2], "started").unwrap();
            thread::sleep(Duration::from_millis(500));
            fs::write(&args[2], "survived").unwrap();
        }
        _ => panic!("unexpected fixture mode"),
    }
}
