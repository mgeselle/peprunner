extern crate serial;

use crate::ssp3::ErrorKind::Protocol;
use serial::core::SerialDevice;
use serial::prelude::*;
use serial::SystemPort;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::time::Duration;
use std::{fmt, io, str};

pub struct Ssp3 {
    port: Box<SystemPort>,
    filter: u8,
    time: u16,
}

#[derive(Debug)]
pub enum ErrorKind {
    Serial(serial::Error),
    Protocol(String),
}

impl From<serial::Error> for ErrorKind {
    fn from(e: serial::Error) -> Self {
        ErrorKind::Serial(e)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorKind::Serial(e) => format!("serial error: {}", e).fmt(f),
            Protocol(e) => format!("protocol error: {}", e).fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        format!("SSP3: {}", self.kind).fmt(f)
    }
}

impl From<serial::Error> for Error {
    fn from(e: serial::Error) -> Self {
        Error { kind: ErrorKind::Serial(e) }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error { kind: ErrorKind::Serial(serial::Error::from(e)) }
    }
}

impl Ssp3 {
    pub fn new<T: AsRef<OsStr> + ?Sized>(port: &T) -> Result<Ssp3, Error> {
        let mut port = serial::open(port)?;
        match port.reconfigure(&|settings| {
            settings.set_baud_rate(serial::Baud19200)?;
            settings.set_char_size(serial::Bits8);
            settings.set_parity(serial::ParityNone);
            settings.set_stop_bits(serial::Stop1);
            Ok(())
        }) {
            Ok(_) => {
                Ok(Ssp3 { port: Box::new(port), filter: 0, time: 0 })
            }
            Err(err) => {
                return Err(Error::from(err));
            }
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.write_with_ack("SSMODE", 2)?;
        self.write_with_ack("SHOME.", 10)?;
        self.filter = 1;
        self.write_with_ack("SGAIN2", 2)
    }

    pub fn measure(&mut self, filter: u8, time: u16) -> Result<u16, Error> {
        if filter == 0 {
            return Err(Error { kind: Protocol("SSP3 not initialized".to_string())})
        }
        if filter > 6 {
            return Err(Error { kind: Protocol(format!("invalid filter {}", filter)) })
        }
        if time >= 6000 {
            return Err(Error { kind: Protocol(format!("invalid time {}", time)) })
        }

        if self.filter != filter {
            self.write_with_ack(&format!("SFILT{:1}", filter), 5)?;
            self.filter = filter;
        }

        if self.time != time {
            self.write_with_ack(&format!("SI{:04}", time), 2)?;
            self.time = time;
        }

        self.write("SCOUNT")?;
        let ioport = self.port.as_mut();
        SerialPort::set_timeout(ioport, Duration::from_secs((time / 100 + 2) as u64))?;

        let mut buffer = [0u8; 9];
        ioport.read_exact(&mut buffer)?;

        let response = str::from_utf8(&buffer).map_err(|e| Error { kind: ErrorKind::Protocol(e.to_string()) } )?;
        if !response.starts_with("C=") {
            return Err(Error { kind: Protocol(format!("invalid response {}", response)) });
        }

        let counts = &response[2..7].parse::<u16>().map_err(|e| Error { kind: ErrorKind::Protocol(e.to_string()) } )?;

        Ok(*counts)
    }

    pub fn finish(&mut self) -> Result<(), Error> {
        if self.filter > 6 {
            return Ok(())
        }
        self.write_with_ack("SEND..", 5).expect("SSP shutdown failed");
        self.filter = 7;
        Ok(())
    }

    fn write_with_ack(&mut self,  output: &str, timeout: u64) -> Result<(), Error> {
        self.write(output)?;
        self.read_ack(output, timeout)
    }

    fn read_ack(&mut self, output: &str, timeout: u64) -> Result<(), Error> {
        let ioport = self.port.as_mut();
        SerialDevice::set_timeout(ioport, Duration::from_secs(timeout))?;

        let mut buffer = [0u8; 3];
        ioport.read_exact(&mut buffer)?;

        if buffer[0] == 0x21 && buffer[1] == 0x0A && buffer[2] == 0x0D {
            return Ok(());
        }

        Err(Error { kind: ErrorKind::Protocol(format!("Received {:?} in response to {}", buffer, output)) })
    }

    fn write(&mut self, output: &str) -> Result<(), Error> {
        let ioport = self.port.as_mut();
        let mut pos = 0;
        let out_buffer = output.as_bytes();
        while pos < output.len() {
            let bytes_written = ioport.write(&out_buffer[pos..])?;
            pos += bytes_written;
        }
        ioport.flush()?;
        Ok(())
    }
}