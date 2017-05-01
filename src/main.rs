#![allow(unused_variables)]

extern crate simplemad;
extern crate portaudio;
#[macro_use]
extern crate error_chain;

use portaudio as pa;
use simplemad::Decoder;
use std::fs::File;
use std::io;
use std::env;

const INTERLEAVED: bool = true;

error_chain! {
    foreign_links {
        PortAudio(pa::Error);
        Io(io::Error);
    }
    errors {
        Simplemad(err: simplemad::SimplemadError) {
            description("something went wrong in simplemad")
            display("{:?}", err)
        }
    }
}

impl From<simplemad::SimplemadError> for Error {
    fn from(err: simplemad::SimplemadError) -> Error {
        ErrorKind::Simplemad(err).into()
    }
}

fn main() {
fn try() -> Result<()> {
    // Open the input file
    let args = env::args();
    let path = args.last().unwrap();
    assert!(path.ends_with(".mp3"));
    let file = File::open(path)?;
    let mut decoder = Decoder::decode(file)?.peekable();

    while let Some(&Err(_)) = decoder.peek() {
        decoder.next();
    }

    let mut frame = decoder.next().ok_or("No frames")??;
    let sample_rate = frame.sample_rate;
    let num_channels = frame.samples.len();

    println!("Sample rate: {}", sample_rate);
    println!("Channels : {}", num_channels);

    let pa = pa::PortAudio::new()?;

    println!("PortAudio");
    println!("version: {}", pa.version());
    println!("version text: {:?}", pa.version_text());
    println!("host count: {}", pa.host_api_count()?);

    let default_host = pa.default_host_api()?;
    println!("default host: {:#?}", pa.host_api_info(default_host));

    let def_output = pa.default_output_device()?;
    let output_info = pa.device_info(def_output)?;
    println!("Default output device info: {:#?}", &output_info);

    // Construct the output stream parameters.
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::<f32>::new(def_output, num_channels as i32, INTERLEAVED, latency);

    // Check that the stream format is supported.
    try!(pa.is_output_format_supported(output_params, sample_rate as f64));

    // Construct the settings with which we'll open our duplex stream.
    let settings = pa::OutputStreamSettings::new(output_params, sample_rate as f64, frame.samples[0].len() as u32);

    let mut stream = pa.open_blocking_stream(settings)?;

    stream.start()?;

    // Now start the main read/write loop! In this example, we pass the input buffer directly to
    // the output buffer, so watch out for feedback.
    'stream: loop {
        // How many frames are available for writing on the output stream?
        let mut out_frames = 0;
        while out_frames < frame.samples[0].len() {
            match stream.write_available()? {
                pa::StreamAvailable::Frames(frames) => {
                    out_frames = frames as usize;
                }
                other => println!("{:?}", other),
            }
        }

        stream.write(out_frames as u32, |output| {
            for i in 0..out_frames {
                for j in 0..num_channels {
                    output[i * num_channels + j] = frame.samples[j][i].to_f32();
                }
            }
            // println!("Wrote {:?} frames to the output stream.", out_frames);
        })?;

        frame = decoder.next().ok_or("No frames")??;
        assert_eq!(sample_rate, frame.sample_rate);
        assert_eq!(num_channels, frame.samples.len());
    }
}
try().unwrap();
}
