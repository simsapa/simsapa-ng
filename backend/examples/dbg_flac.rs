use simsapa_backend::audio::recorder::encode_canonical_pcm_to_flac;
use simsapa_backend::audio::format::CANONICAL_SAMPLE_RATE;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
fn main() {
    let n = (CANONICAL_SAMPLE_RATE as f32 * 1.0) as usize;
    let mut pcm: Vec<i16> = (0..n).map(|i| {
        let t = i as f32 / CANONICAL_SAMPLE_RATE as f32;
        ((2.0*std::f32::consts::PI*440.0*t).sin()*0.5*i16::MAX as f32) as i16
    }).collect();
    // Pad to a multiple of the FLAC block size (4096) so every frame is full
    // length and STREAMINFO min_blocksize == max_blocksize.
    let bs = 4096;
    let rem = pcm.len() % bs;
    if rem != 0 { pcm.resize(pcm.len() + (bs - rem), 0); }
    println!("padded len={}", pcm.len());
    let p = std::path::Path::new("/tmp/dbg.flac");
    encode_canonical_pcm_to_flac(&pcm, p).unwrap();
    let bytes = std::fs::read(p).unwrap();
    println!("len={} first4={:02x?}", bytes.len(), &bytes[..4]);
    let file = std::fs::File::open(p).unwrap();
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("flac");
    match symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default()) {
        Ok(probed) => {
            let t = probed.format.default_track().unwrap();
            println!("OK rate={:?} ch={:?}", t.codec_params.sample_rate, t.codec_params.channels);
        }
        Err(e) => println!("PROBE ERR: {:?}", e),
    }
}
