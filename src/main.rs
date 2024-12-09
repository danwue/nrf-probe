mod deframer;
mod mul;
mod nrf_stat_sink;
mod stdout_sink;
mod union;

use deframer::{NrfConfig, NrfDeframer, NrfFrame};
use mul::Multiply;
use nrf_stat_sink::NrfStatSink;
use rustradio::{graph::GraphRunner, stream::Streamp, Error};
use rustradio::{mtgraph::MTGraph, stream::NoCopyStreamp, window::WindowType, Complex};
use std::iter::zip;
use stdout_sink::StdoutSink;
use structopt::{
    clap::{crate_authors, crate_description, crate_name},
    StructOpt,
};

use rustradio::blocks::*;
use union::Union;

macro_rules! add_block {
    ($g:ident, $cons:expr) => {{
        let block = Box::new($cons);
        let prev = block.out();
        $g.add(block);
        prev
    }};
}

fn range_validator(min: u8, max: u8) -> impl Fn(std::string::String) -> Result<(), String> {
    move |value: String| {
        if (min..=max).map(|x| x.to_string()).any(|x| x == value) {
            Ok(())
        } else {
            let err = format!("Must be in range [{}, {}]", min, max);
            Err(err)
        }
    }
}

#[derive(StructOpt, Debug)]
#[structopt(author = crate_authors!(), name = crate_name!(), about = crate_description!())]
struct Opt {
    /// SoapySDR driver name
    #[structopt(short = "d", long = "driver", default_value = "hackrf")]
    driver: String,

    /// Channel selection, must be within range [1,125]
    #[structopt(short = "c", long = "channel", min_values = 1, required = true, validator = range_validator(0, 125), use_delimiter = true)]
    channels: Vec<u8>,

    /// Input gain in dBi
    #[structopt(short = "g", long = "gain", default_value = "20")]
    gain: i32,

    /// Sample rate in MHz
    #[structopt(short = "s", long = "sample")]
    sample_rate_mhz: Option<u8>,

    /// Address length in bytes
    #[structopt(short = "a", long = "alen", default_value = "5", possible_values = &["3", "4", "5"])]
    address_length: usize,

    /// Hexadecimal prefix of the address
    #[structopt(short = "p", long = "address")]
    address_prefix: Option<String>,

    /// Payload length in bytes, must be within range [0,32]
    #[structopt(short = "l", long = "plen", validator = range_validator(0,32), required_unless("shockburst"))]
    payload_length: Option<usize>,

    /// Support for Enhanced ShockBurst (ESB) packet headers, allows dynamic payload lengths
    #[structopt(short = "e", long = "shockburst")]
    shockburst: bool,

    /// Data rate (1Mpbs or 2Mpbs)
    #[structopt(short="r", long = "rate", default_value = "1", possible_values = &["1", "2"])]
    rate: u8,

    /// Prints statistics about received packets instead of full packet payloads
    #[structopt(long = "stats")]
    stats: bool,
}

impl Opt {
    // returns highest and lowest channel
    fn min_max_channel(&self) -> (u8, u8) {
        let mut channels = self.channels.clone();
        channels.sort();
        (channels[0], channels[channels.len() - 1])
    }

    fn sample_rate(&self) -> f32 {
        let (min, max) = self.min_max_channel();
        self.sample_rate_mhz
            .map_or((max - min) as f32 + 2.0, |s| s as f32)
            * 1_000_000.0
    }

    fn center_freq(&self) -> f32 {
        let (min, max) = self.min_max_channel();
        2_400_000_000.0 + 1_000_000.0 * (min + max) as f32 / 2.0
    }

    fn address_prefix_bytes(&self) -> Vec<u8> {
        self.address_prefix.clone().map_or(vec![], |prefix| {
            hex::decode(prefix).expect("Could not parse address prefix")
        })
    }
}

fn process_channel(
    graph: &mut MTGraph,
    opt: &Opt,
    center_freq: f32,
    input: Streamp<Complex>,
    channel: u8,
) -> NoCopyStreamp<NrfFrame> {
    let channel_freq = 2_400_000_000.0 + 1_000_000.0 * channel as f32;

    let shifted = if channel_freq == center_freq {
        input
    } else {
        let shift_source = add_block!(
            graph,
            SignalSourceComplex::new(opt.sample_rate(), center_freq - channel_freq, 1.0)
        );
        add_block!(graph, Multiply::new(input, shift_source))
    };

    let low_pass = add_block!(
        graph,
        FftFilter::new(
            shifted,
            &rustradio::fir::low_pass_complex(
                opt.sample_rate(),
                1_000_000.0, // cut-off: 1M
                250_000.0,   // twidth: 250k
                &WindowType::Hamming,
            ),
        )
    );

    let quad_demod = add_block!(graph, QuadratureDemod::new(low_pass, 1.0));

    let clock_recovery = add_block!(
        graph,
        ZeroCrossing::new(
            quad_demod,
            opt.sample_rate() / (opt.rate as f32 * 1_000_000.0), // Samples per symbol
            0.0,                                                 // Max deviation (unused)
        )
    );

    let bin_slice = add_block!(graph, BinarySlicer::new(clock_recovery));

    let config = if opt.shockburst {
        NrfConfig::shockburst(
            channel,
            opt.address_length,
            opt.payload_length,
            &opt.address_prefix_bytes(),
        )
    } else {
        NrfConfig::fixed_length(
            channel,
            opt.address_length,
            opt.payload_length.expect(
                "Either Enhanced ShockBurst needs to enabled or payload length must be defined.",
            ),
            &opt.address_prefix_bytes(),
        )
    };

    add_block!(graph, NrfDeframer::new(bin_slice, config))
}

pub fn main() -> Result<(), Error> {
    let options = Opt::from_args();

    let mut graph = MTGraph::new();

    eprintln!(
        "Selected center frequency: {} MHz",
        options.center_freq() / 1_000_000.0
    );
    eprintln!(
        "Selected sample rate: {} MHz",
        options.sample_rate() / 1_000_000.0
    );

    let mut sources = vec![add_block!(
        graph,
        SoapySdrSourceBuilder::new(
            options.driver.clone(),
            options.center_freq() as f64,
            options.sample_rate() as f64
        )
        .igain(options.gain as f64)
        .build()?
    )];

    // split source for each channel
    while sources.len() < options.channels.len() {
        if let Some(source) = sources.pop() {
            let (a, b) = add_block!(graph, Tee::new(source));
            sources.push(a);
            sources.push(b);
        }
    }

    // process individual channels
    let processed: Vec<NoCopyStreamp<NrfFrame>> = zip(sources, options.channels.iter().copied())
        .map(|(source, channel)| {
            process_channel(&mut graph, &options, options.center_freq(), source, channel)
        })
        .collect();

    // union all received messages
    let union = processed
        .into_iter()
        .reduce(|a, b| add_block!(graph, Union::new(a, b)))
        .expect("At least one channel must be provided");

    // output receives messages
    if options.stats {
        graph.add(Box::new(NrfStatSink::new(union)));
    } else {
        graph.add(Box::new(StdoutSink::new(union)));
    }

    let cancel = graph.cancel_token();
    ctrlc::set_handler(move || {
        eprintln!("\n");
        cancel.cancel();
    })
    .expect("Failed to set Ctrl-C handler");

    let st = std::time::Instant::now();
    graph.run()?;
    eprintln!("{}", graph.generate_stats(st.elapsed()));
    Ok(())
}
