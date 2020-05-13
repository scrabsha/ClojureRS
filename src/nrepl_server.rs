//! Creates and maintain a simple NREPL server.

use std::{
    collections::HashMap,
    dbg,
    io::{BufReader, Read, Result as IResult, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    str,
};

use rand::{self, Rng};

use bendy::{
    decoding::{Decoder, DictDecoder, Error as DError, Object},
    encoding::{Error as EError, SingleItemEncoder, ToBencode},
};

use crate::repl::Repl;

const SERVER_PORT: u16 = 5555;

/// Returns the address of the server.
fn get_address(port: u16) -> SocketAddrV4 {
    let lh = Ipv4Addr::LOCALHOST;
    SocketAddrV4::new(lh, port)
}

// TODO: add a nice way to change the server port instead of always using the
// default one.
fn nrepl_default_address() -> SocketAddrV4 {
    get_address(SERVER_PORT)
}

// TODO: switch to something better, perhaps five usize or something else.
type SessionId = String;

pub struct Server {
    sessions: HashMap<SessionId, Repl>,
}

impl Server {
    /// Creates a new server and runs it.
    pub fn run() -> IResult<()> {
        let addr = nrepl_default_address();
        let listener = TcpListener::bind(addr)?;

        let (mut stream, addr) = listener.accept()?;

        println!("Starting server...");
        let mut server = Server::new();

        loop {
            let mut buffer = [0; 512];
            let bytes_received = stream.read(&mut buffer)?;

            if bytes_received == 0 {
                // If nothing was ridden from the client, then the connection
                // ended.
                break;
            } else if bytes_received == 512 {
                // If we panic there, then it might be good to increase the
                // buffer size.
                panic!("Request was too big!");
            }

            let (to_decode, _) = buffer.split_at(bytes_received);

            let dec = Decoder::new(to_decode);
            let req = decode_request(dec).expect("Request decoding failed");

            let to_send = server
                .run_request(req)
                .expect("Request execution failed")
                .to_bencode()
                .expect("Response encoding failed");

            stream.write(to_send.as_slice())?;
        }

        Ok(())
    }

    /// Creates a new server.
    fn new() -> Server {
        Server {
            sessions: HashMap::new(),
        }
    }

    /// Runs a request, updates the inner state, and returns the data that
    /// should be returned to the client.
    fn run_request(&mut self, r: Request) -> Result<Response, RequestError> {
        match r {
            Request::Clone(id) => {
                self.sessions.insert(id.clone(), Repl::default());
                let new_session = random_uuid();
                let status = "done";
                Ok(Response::Cloned {
                    id,
                    new_session,
                    status,
                })
            }
        }
    }
}

/// A response generated by the server.
enum Response {
    /// Emitted when a `Clone` was requested
    Cloned {
        id: SessionId,
        new_session: SessionId,
        status: &'static str,
    },
}

impl ToBencode for Response {
    const MAX_DEPTH: usize = 1;

    fn encode(&self, enc: SingleItemEncoder) -> Result<(), EError> {
        match self {
            Response::Cloned {
                id,
                new_session,
                status,
            } => enc.emit_dict(|ref mut dict_encoder| {
                dict_encoder.emit_pair(b"id", id.as_str())?;
                dict_encoder.emit_pair(b"new-session", new_session.as_str())?;
                dict_encoder.emit_pair(b"status", status)
            }),
        }
    }
}

/// A request, raised by `decode_request`.
enum Request {
    /// When the client wants to clone a session.
    Clone(String),
}

/// An error raised by the server when it couldn't handle a request from a
/// client.
#[derive(Debug)]
enum RequestError {
    /// Generated when bencode decoding fails.
    ///
    /// The inner data is generated by `bendy`.
    DError(DError),
    /// Generated when the input data is not a dictionary.
    UnexpectedObject,
    /// Generated a string is expected, but something else is given.
    FailedToReadValue,
    /// Raised when the input dictionary does not contain any `op` key.
    Noop,
    /// Raised when the `op` key is not recognized.
    UnknownOp,
    /// Raised when a key should be present, but was not supplied by the client.
    KeyNotFound(&'static str),
}

impl From<DError> for RequestError {
    fn from(e: DError) -> RequestError {
        RequestError::DError(e)
    }
}

fn decode_request(mut input: Decoder) -> Result<Request, RequestError> {
    let object = input.next_object()?.unwrap();

    let request_dict = match object {
        Object::Dict(d) => decode_dict(d),
        _ => Err(RequestError::UnexpectedObject),
    }?;

    let mut op = request_dict
        .get("op")
        .map(String::as_str)
        .ok_or(RequestError::Noop)?;

    match op {
        "clone" => request_dict
            .get("id")
            .map(|i| Request::Clone(i.into()))
            .ok_or(RequestError::KeyNotFound("id")),
        _ => {
            dbg!(request_dict);
            Err(RequestError::UnknownOp)
        }
    }
}

/// Turns a request from the client into a hashmap.
///
/// # Errors
///
/// This function will return an error if the decoding step fails.
///
/// # Safety
///
/// This function will panic if the input contains non-utf8 characters.
fn decode_dict(mut d: DictDecoder) -> Result<HashMap<String, String>, RequestError> {
    let mut dict_map = HashMap::new();

    while let Some((k, v)) = d.next_pair()? {
        let k = str::from_utf8(k).expect("Conversion to UTF8 failed");

        let v = match v {
            Object::Bytes(b) => str::from_utf8(b).expect("Conversion to UTF8 failed"),
            _ => return Err(RequestError::FailedToReadValue),
        };

        dict_map.insert(k.into(), v.into());
    }

    Ok(dict_map)
}

// Adapted from random-uuid from clojurescript
fn random_uuid() -> String {
    let base = 16_usize;

    let mut rng = rand::thread_rng();

    let mut hex_num = |n| rng.gen_range(0, base.pow(n));

    let part_1 = hex_num(8);
    let part_2 = hex_num(4);

    let part_3_to_add = 4 * base.pow(3);
    let part_3 = part_3_to_add + hex_num(3);

    let rhex = 8 | (3 & hex_num(1));
    let part_4 = rhex + hex_num(3);

    let part_5 = hex_num(12);

    // Note: this use of format is necessary because we need to print each
    // digit group in hexadecimal.
    //
    // This function may be refactored in order to avoid its use, but it is
    // not strictly necessary as it is not a hot path.
    format!(
        "{:x}-{:x}-{:x}-{:x}-{:x}",
        part_1, part_2, part_3, part_4, part_5
    )
}
