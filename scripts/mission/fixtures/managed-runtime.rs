// Owned synthetic startup server, never a model or linguistic quality oracle.
use std::io::{Read,Write};
fn main(){
 let args:Vec<String>=std::env::args().collect();
 let val=|key:&str|args[args.iter().position(|a|a==key).unwrap()+1].clone();
 let root=std::path::PathBuf::from(val("-m")).parent().unwrap().to_owned();
 std::fs::write(root.join("runtime.pid"),std::process::id().to_string()).unwrap();
 std::fs::write(root.join("runtime.cwd"),std::env::current_dir().unwrap().to_str().unwrap()).unwrap();
 let listener=std::net::TcpListener::bind(("127.0.0.1",val("--port").parse::<u16>().unwrap())).unwrap();
 for incoming in listener.incoming(){
  let Ok(mut stream)=incoming else {continue};
  let _=(||->std::io::Result<()> {
  let mut data=Vec::new();let mut byte=[0];
  while !data.ends_with(b"\r\n\r\n") && data.len()<32768 {if stream.read(&mut byte)?==0 {break;}data.push(byte[0]);}
  let headers=String::from_utf8_lossy(&data).to_ascii_lowercase();
  let length=headers.lines().find_map(|l|l.strip_prefix("content-length:").map(|n|n.trim().parse::<usize>().unwrap())).unwrap_or(0);
  assert!(length<=32768);let mut body=vec![0;length];stream.read_exact(&mut body)?;
  let mode=std::fs::read_to_string(root.join("phase-control")).unwrap_or_else(|_|"wait".into());
  let mut status="200 OK";let mut response="{}";
  if data.starts_with(b"GET /health") && mode=="wait" {status="503 Service Unavailable";}
  if data.starts_with(b"GET /slots") {response="[{\"id\":0,\"is_processing\":false}]";}
  if data.starts_with(b"POST") {
   let name=if String::from_utf8_lossy(&body).contains("\"max_tokens\":256") {"user-requests"} else {"warmups"};
   let path=root.join(name);let n=std::fs::read_to_string(&path).ok().and_then(|v|v.parse::<u32>().ok()).unwrap_or(0);
   std::fs::write(path,(n+1).to_string()).unwrap();
   if mode=="ready" {response="{\"choices\":[{\"finish_reason\":\"length\",\"message\":{\"content\":\"O\"}}],\"usage\":{\"completion_tokens\":1}}";}
  }
  let reply=format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len());
  stream.write_all(reply.as_bytes())?;
  Ok(())
  })(); // Health polling may abandon a socket; keep serving subsequent requests.
 }
}
