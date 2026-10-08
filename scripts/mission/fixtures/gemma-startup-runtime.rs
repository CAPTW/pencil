use std::io::{Read,Write};
fn main(){
 let args:Vec<String>=std::env::args().collect();
 let val=|key:&str|args[args.iter().position(|a|a==key).unwrap()+1].clone();
 let model=std::path::PathBuf::from(val("-m"));
 std::fs::write(model.with_extension("cwd"),std::env::current_dir().unwrap().to_str().unwrap()).unwrap();
 std::fs::write(model.with_extension("pid"),std::process::id().to_string()).unwrap();
 let mode=std::fs::read_to_string(&model).unwrap();
 let listener=std::net::TcpListener::bind(("127.0.0.1",val("--port").parse::<u16>().unwrap())).unwrap();
 for incoming in listener.incoming(){
  let mut stream=incoming.unwrap();let mut data=Vec::new();let mut byte=[0];
  while !data.ends_with(b"\r\n\r\n") && data.len()<32768 {if stream.read(&mut byte).unwrap()==0 {break;}data.push(byte[0]);}
  // Consume the complete request before closing; unread POST bytes cause a TCP
  // reset on Windows and would test transport failure instead of invalid JSON.
  let headers=String::from_utf8_lossy(&data).to_ascii_lowercase();
  let length=headers.lines().find_map(|line|line.strip_prefix("content-length:").map(|n|n.trim().parse::<usize>().unwrap())).unwrap_or(0);
  assert!(length<=32768);
  let mut body=vec![0;length];stream.read_exact(&mut body).unwrap();
  if data.starts_with(b"POST") {
   std::fs::write(model.with_extension("warmup"),b"reached").unwrap();
   if mode=="timeout" {std::thread::sleep(std::time::Duration::from_secs(30));}
  }
  let _=stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
 }
}
