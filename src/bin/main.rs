use server::{ HttpResponse, ThreadPool, Config };
use std::time::{ SystemTime, UNIX_EPOCH };
use std::fs;
use std::env;
use std::io::prelude::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;

const IP: &str = "127.0.0.1";

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args);

    let addr = format!("{}:{}", IP, config.port);
    println!("server address is: {}", addr);

    let listener = TcpListener::bind(addr).unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let path = config.root_path.clone();

        pool.execute(move || {
            handle_connection(stream, path);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream, root_path: String) {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let time_in_ms = start.as_secs() * 1000 +
        start.subsec_nanos() as u64 / 1_000_000;
    let mut file = OpenOptions::new().append(true).open("logs.txt").expect("cannot open file");

    let bad_req_err_res = HttpResponse {
        status_line: "HTTP/1.1 400 BAD REQUEST".to_string(),
        template: "400.html".to_string(),
    };

    let not_found_err_res = HttpResponse {
        status_line: "HTTP/1.1 404 NOT FOUND".to_string(),
        template: "404.html".to_string(),
    };

    let not_implemented_err_res = HttpResponse {
        status_line: "HTTP/1.1 501 NOT IMPLEMENTED".to_string(),
        template: "501.html".to_string(),
    };

    let ok_res = HttpResponse {
        status_line: "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Disposition: \
        attachment; filename = test.txt\r\n\r\n".to_string(),
        template: "test.txt".to_string(),
    };

    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    println!("{}",  String::from_utf8_lossy(&buffer));

    let get = format!("GET {} HTTP/1.1\r\n", root_path);
    let main_page = buffer.starts_with(get.as_ref());

    let post = buffer.starts_with(b"POST");
    let patch = buffer.starts_with(b"PUTCH");
    let put = buffer.starts_with(b"PUT");
    let delete = buffer.starts_with(b"DELETE");
    let status_line: String;
    let filename: String;

    if post || patch || put || delete {
        status_line = not_implemented_err_res.status_line;
        filename = not_implemented_err_res.template;
    } else if main_page {
        status_line = ok_res.status_line;
        filename = ok_res.template;
    } else {
        if buffer.starts_with(b"GET") {
            status_line = not_found_err_res.status_line;
            filename = not_found_err_res.template;
        } else {
            status_line = bad_req_err_res.status_line;
            filename = bad_req_err_res.template;
        }
    }

    let contents = fs::read_to_string(filename).unwrap();

    file.write_all(format!("\n{}:{}", time_in_ms, status_line).as_bytes()).expect("write failed");
    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    );

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
