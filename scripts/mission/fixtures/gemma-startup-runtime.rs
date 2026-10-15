use std::io::{Read,Write};
fn main(){
 let args:Vec<String>=std::env::args().collect();
 let val=|key:&str|args[args.iter().position(|a|a==key).unwrap()+1].clone();
 let model=std::path::PathBuf::from(val("-m"));
 let panic_receipt=model.with_extension("fixture-error");
 std::panic::set_hook(Box::new(move |info| {
  // Synthetic fixture diagnostics only; no production response/source logging.
  let _=std::fs::write(&panic_receipt,info.to_string());
 }));
 std::fs::write(model.with_extension("cwd"),std::env::current_dir().unwrap().to_str().unwrap()).unwrap();
 std::fs::write(model.with_extension("pid"),std::process::id().to_string()).unwrap();
 let mode=std::fs::read_to_string(&model).unwrap();
 let listener=std::net::TcpListener::bind(("127.0.0.1",val("--port").parse::<u16>().unwrap())).unwrap();
 'connections: for incoming in listener.incoming(){
  let Ok(mut stream)=incoming else {continue;};let mut data=Vec::new();let mut byte=[0];
  stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
  while !data.ends_with(b"\r\n\r\n") && data.len()<32768 {
   // Health polling may cancel a connection; that must not kill the fixture.
   match stream.read(&mut byte) {Ok(1)=>data.push(byte[0]),_=>continue 'connections}
  }
  if !data.ends_with(b"\r\n\r\n") {continue;}
  // Consume the complete request before closing; unread POST bytes cause a TCP
  // reset on Windows and would test transport failure instead of invalid JSON.
  let headers=String::from_utf8_lossy(&data).to_ascii_lowercase();
  let length=headers.lines().find_map(|line|line.strip_prefix("content-length:").map(|n|n.trim().parse::<usize>().unwrap())).unwrap_or(0);
  assert!(length<=32768);
  let mut body=vec![0;length];if stream.read_exact(&mut body).is_err() {continue;}
  if data.starts_with(b"POST") {
   std::fs::write(model.with_extension("warmup"),b"reached").unwrap();
   if mode=="timeout" {std::thread::sleep(std::time::Duration::from_secs(30));}
  }
  let _=stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
 }
}
