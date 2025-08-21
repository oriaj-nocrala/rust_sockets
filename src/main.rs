use bincode::error::EncodeError;
use bincode;
use std::{error, fs};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug, bincode::Encode, bincode::Decode)]
struct Mensaje{
    tipo: Tipo,
    filename: Option<String>,
    mensaje: Vec<u8>
}

#[derive(Debug, bincode::Encode, bincode::Decode)]
enum Tipo{
    Texto,
    Archivo
}

// impl Display for Tipo{
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self{
//             Self::Texto => write!(f, "0"),
//             Self::Archivo => write!(f, "1"),
//         }
//     }
// }

impl Mensaje{
    fn try_to_bytes(&self) -> Result<Vec<u8>, EncodeError> {
        return bincode::encode_to_vec(self, bincode::config::standard());
    }
}



fn handle_client(stream: &mut TcpStream) -> Result<(), Box<dyn error::Error>> {
    println!("Cliente conectadini");
    let mut size_bytes = [0; 8];

    stream.read_exact(&mut size_bytes)?;
    let size = usize::from_be_bytes(size_bytes);

    println!("Esperando recibir {} bytes...", size);
    
    let mut received = vec![0; size];
    stream.read_exact(&mut received)?;
    println!("Recibidos {}", received.len());

    let decoded: Mensaje = bincode::decode_from_slice(&received, bincode::config::standard()).unwrap().0;
    match decoded.tipo{
        Tipo::Texto => {
            println!("Mensaje recibido: {}", String::from_utf8(decoded.mensaje)?);
        }
        Tipo::Archivo => {
            let dir = "recibidos";
            println!("{}{}",std::env::current_dir()?.to_string_lossy(), "/recibidos");
            if !fs::metadata(dir).is_ok(){
                _ = fs::create_dir(dir)?;
            }
            let path = format!("{}/{}", dir, decoded.filename.unwrap());
            let mut buffer = fs::File::create_new(&path)?;
            _ = buffer.flush();
            match buffer.write(&decoded.mensaje){
                Ok(sz) => {
                    println!("Escritos {}", sz);
                },
                Err(e) => { println!("Error: {}", e)}
            };
            // println!("Escritos {}", sz?)
        }
    }
    Ok(())

}

fn read_console_int() -> Result<i32, Box<dyn error::Error>>{
    let _ = io::Write::flush(&mut io::stdout());
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    let res: i32 = buffer.trim().parse()?;
    Ok(res)
}

fn main() -> std::io::Result<()> {
    let local_ip = local_ip_address::local_ip().unwrap().to_string(); // Obtener la IP local
    println!("La ip local es: {}", local_ip);
    
    print!("Enviar (0) o recibir (1)? 0,1: ");
    let val: i32 = read_console_int().expect("No es un numero");

    match val{
        0 => {
            print!("Ingrese la direccion IP: ");
            let _ = io::Write::flush(&mut io::stdout());
            let mut addr = String::new();
            io::stdin().read_line(&mut addr)?;
            let addr = addr.trim();
            if let Ok(mut stream) = TcpStream::connect(format!("{}:6969",addr)){
                println!("Conectadini");
                print!("Eviar mensaje(0) o Enviar archivo (1): ");
                let val: i32 = read_console_int().expect("No es un numero");

                match val{
                    0 => {
                        print!("Ingrese mensaje: ");
                        let _ = io::Write::flush(&mut io::stdout());
                        let mut msg = String::new();
                        io::stdin().read_line(&mut msg)?;
                        let msg: Mensaje = Mensaje{ tipo: Tipo::Texto, filename: None, mensaje: msg.into_bytes() };
                        let archivo_bytes: &Vec<u8> = &msg.try_to_bytes().unwrap();
                        let size = archivo_bytes.len();
                        stream.write(&size.to_be_bytes())?;
                        stream.flush()?;
                        stream.write(archivo_bytes)?;
                        stream.flush()?;
                        println!("Enviando {} bytes", size);
                    },
                    1 => {
                        print!("Nombre del archivo: ");
                        let _ = io::Write::flush(&mut io::stdout());
                        let mut msg = String::new();
                        io::stdin().read_line(&mut msg)?;
                        let msg = msg.trim();
                        // dbg!("El archivo es: {}", &msg);
                        if fs::metadata(&msg).is_ok(){
                            match fs::read(&msg){
                                Ok(file) => {
                                    let msg: Mensaje = Mensaje{ tipo: Tipo::Archivo, filename: Some(msg.to_string()), mensaje: file };
                                    let archivo_bytes: &Vec<u8> = &msg.try_to_bytes().unwrap();
                                    let size = archivo_bytes.len();
                                    stream.write(&size.to_be_bytes())?;
                                    stream.flush()?;
                                    stream.write(archivo_bytes)?;
                                    stream.flush()?;
                                    println!("Escritos {} bytes", size);
                                },
                                Err(e) => println!("Error: {}", e)
                            }
                        } else {
                            println!("No se encontro el archivo");
                        }
                    },
                    _ => {
                        println!("Valor invalido");
                    }
                }
                

                let _ = stream.flush();
            } else {
                println!(" ERROR CTM ");
            }
        },
        1 => {
            println!("Escuchando en el puerto 6969");
            let listener = TcpListener::bind("0.0.0.0:6969")?;
    
            // accept connections and process them serially
            for stream in listener.incoming() {
                let _ = handle_client(&mut stream?).unwrap_or_else(|e| println!("Error al recibir mensajito {}", e));
            }
        },
        _ => {}
    }
    Ok(())
}