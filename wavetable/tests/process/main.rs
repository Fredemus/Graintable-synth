extern crate hound;

use wavetable::Synth;

const SAMPLES: &'static [&'static [u8]] = &[&[60], &[41, 48], &[77, 80, 84]];

fn process_note_samples_mono(notes: &[u8], samples: usize) -> [Vec<f32>; 1] {
    let mut plugin = Synth::default();
    let mut outputs = [vec![0.0; samples]];
    let mut buffer = outputs.iter_mut().map(|buf| &mut buf[..]);
    for &note in notes {
        plugin.note_on(note);
    }
    plugin.process(samples, &mut buffer);
    outputs
}

fn _process_note_samples_stereo(notes: &[u8], samples: usize) -> [Vec<f32>; 2] {
    let mut plugin = Synth::default();
    let mut outputs = [vec![0.0; samples], vec![0.0; samples]];
    let mut buffer = outputs.iter_mut().map(|buf| &mut buf[..]);
    for &note in notes {
        plugin.note_on(note);
    }
    plugin.process(samples, &mut buffer);
    outputs
}
// simple test for debugging. used for checking if a variable has the expected value
// has to be run with cargo test -- --nocapture or the println! will be suppressed
#[test]
fn test_variable_value() {
    let _plugin = Synth::default();
}

#[test]
fn test_process_mono() {
    for notes in SAMPLES.iter() {
        let stem = format!(
            "sample-{}",
            notes
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join("-")
        );
        let file = format!("{}/tests/process/{}.wav", env!("CARGO_MANIFEST_DIR"), stem);
        println!("{}", file);
        let reader = hound::WavReader::open(file).unwrap();
        let [output] = process_note_samples_mono(notes, 44100);

        assert!(reader
            .into_samples::<f32>()
            .map(|sample| sample.expect("failed to decode WAV stream"))
            .zip(output)
            .all(|(exp, out)| {
                println!("{:?} == {:?}", out, exp);
                out == exp
            }));
    }
}

#[test]
#[ignore] // FIXME(will): tests with fs side-effects are a little weird but this works for now
fn write_test_samples_mono() {
    for notes in SAMPLES.iter() {
        let stem = format!(
            "sample-{}",
            notes
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join("-")
        );
        let file = format!("{}/tests/process/{}.wav", env!("CARGO_MANIFEST_DIR"), stem);
        println!("{}", file);
        let [output] = process_note_samples_mono(notes, 44100);
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&file, spec).unwrap();
        for t in 0..44100 {
            writer.write_sample(output[t]).unwrap();
        }
        writer.finalize().unwrap();
    }
}
