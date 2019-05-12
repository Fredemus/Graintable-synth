# Graintable-synth
WIP vst synthesizer written in rust.

The source code is in lib.rs. Cargo.toml lists dependencies.

The goal for now is to implement a wavetable oscillator that can be controlled by midi.

Part goals needed:
* Note on/off ✓
* wavetable pos ✓
* Let max wavetable pos change dynamically with reader.duration ✓
* Resample function to change pitch (https://ccrma.stanford.edu/~jos/resample/) (http://yehar.com/blog/wp-content/uploads/2009/08/deip.pdf) ✓
* way to slice wavetable into individual waveforms (to avoid interpolation glitches) ✓
* FIR Filter for oversampling (2x oversampling should be fine) ✓


