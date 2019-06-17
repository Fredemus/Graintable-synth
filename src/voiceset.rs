use std::collections::VecDeque;
use std::f32;
mod filter; 
pub mod interp; 
mod modmatrix;
/*
        Todo: 
        the stuff to force envelope properly into release state doesn't work

        small alias problem now. SNR at 1 kHz is about -80 dB.
        Most likely caused by quality of interpolation algorithm

        Optimization ideas : flatten vectors(possibly big improvement, way fewer cache misses. Nalgebra has tools),
        iterate instead of index (should be ~20% faster),
        possibly change some vectors to arrays (could be done instead of flattening, easier).
        the actual samples per waveform, and number of mip maps is known at compile-time.
        Number of waveforms is not

*/
#[allow(dead_code)]
pub struct Voiceset {
    pub(crate) oscs: Vec<interp::WaveTable>,
    pub(crate) g_oscs : Vec<interp::GrainTable>,
    //pub(crate) osc2 : Interp,
    //tweakable synth parameters
    pub vol: Vec<f32>,
    pub detune: Vec<f32>,
    //pub osc2_vol : f32, pub det2 : f32,
    pub voice: Vec<Voice>,
    //vector of filters, since each voice will need its own filter when envelopes are added
    pub filter: Vec<filter::DecentFilter>,
    //interp_buffer gives room for filtering continuous output from oscillator.
    pub(crate) interp_buffer: VecDeque<f32>,
    pub pos: Vec<usize>,
    pub octave: Vec<i8>,
    pub vol_env : modmatrix::Env,
    pub mod_env : modmatrix::Env,
}
impl Voiceset {
    //might require more antialiasing
    pub fn step_one(&mut self) -> f32 {
        let mut output = 0.;
        //needs to have a way to go through all unison voices
        //downsampling for loop
        for i in 0..self.oscs[0].amt_oversample {
            let mut unfiltered_new = 0.;
            for voice in 0..8 {
                let modboy = self.vol_env.next(voice);
                //add the output of the active voices together
                if modboy == None { //if env is none, it's done outputting
                    //break;
                    self.voice[voice].reset_its();
                } 
                else {
                    let mut temp = 0.;
                    
                    for osc in 0..2 {
                        //the 2 oscillators
                        if modboy != None {
                            temp += self.single_interp(
                            self.voice[voice].ratio * self.detune[osc],
                            voice,
                            osc,
                            ) * self.vol[osc] * modboy.unwrap(); 
                        }
                        else {
                            temp += self.single_interp(
                            self.voice[voice].ratio * self.detune[osc],
                            voice,
                            osc,
                            ) * self.vol[osc]; 
                        }
                        
                    }
                    self.filter[voice].tick_pivotal(temp, None);
                    //self.filter[voice].tick_pivotal(temp);
                    unfiltered_new += self.filter[voice].vout[self.filter[0].poles];
                }
            }
            
            //only every 2nd sample needs to be output for downsampling. Therefore only every 2nd sample
            //needs to be filtered
            if i % 2 == 0 {
                output = self.single_convolve(&self.oscs[0].downsample_fir);
                //output = self.interp_buffer[self.oscs[0].downsample_fir.len()];
            }
            //removes the sample that just got filtered
            self.interp_buffer.pop_front();
            //adds a new unfiltered sample to the end
            self.interp_buffer.push_back(unfiltered_new);
        }
        return output;
    }
    //needs looping by grain size and grain size in the ratio
    pub fn _single_interp_grain(&mut self, ratio : f32, i: usize, j : usize) -> f32 {
        let mip = (self.voice[i].current_mip as i8 + self.octave[j]) as usize; /*(1./ratio).log2().floor() as usize;*/
        //let downsampled_ratio = 2f32.powi(self.voice[i].current_mip as i32);
        let len = self.g_oscs[j].mips[mip].len();
        let offset = self.pos[j] * len;
        let mut temp: f32;
        let it: usize;
        let x = ratio * self.g_oscs[j].grain_size / len as f32;
        let z_pos; //= z.fract();
        it = self.voice[i].unison_its[j][0].floor() as usize; 

        z_pos = self.voice[i].unison_its[j][0].fract();
        temp = ((self.g_oscs[j].c3[mip][it + offset] * z_pos 
            + self.g_oscs[j].c2[mip][it + offset]) * z_pos
            + self.g_oscs[j].c1[mip][it + offset]) * z_pos 
            + self.g_oscs[j].c0[mip][it + offset];
        
        self.voice[i].unison_its[j][0] += x;
        //loop from the grain size:
        if self.voice[i].unison_its[j][0] > offset as f32 + self.g_oscs[j].grain_size / 4096. {
            //loop back around zero.
            self.voice[i].unison_its[j][0] -= self.g_oscs[j].grain_size / 4096.;
        }


        if self.voice[i].unison_its[j][0] > (len) as f32 {
            //loop back around zero.
            self.voice[i].unison_its[j][0] -= (len) as f32;
        }
        //apply a window to the grain to declick it: 
        temp = temp * ((1./4095.)*3.14151592*self.voice[i].unison_its[j][0]).sin();
        return temp;
    }

    

    pub(crate) fn single_interp(&mut self, ratio: f32, i: usize, j: usize) -> f32 {
        // Optimal 2x (4-point, 3rd-order) (z-form)
        // return ((c3*z+c2)*z+c1)*z+c0;
        //find the best mip to do the interpolation from. could be moved elsewhere to avoid calling excessively
        let mip = (self.voice[i].current_mip as i8 + self.octave[j]) as usize; 
        let temp: f32;
        let it: usize;
        //x is the placement of the sample compared to the last one, or the slope
        let x = ratio;
        //self.new_len = findlen as usize;
        //let z = x - 0.5;
        let z_pos; //= z.fract();
        it = self.voice[i].unison_its[j][0].floor() as usize; //have a way to use each unison it in use
                                                              //should z_pos have a -0.5?
        z_pos = self.voice[i].unison_its[j][0].fract();
        temp = ((self.oscs[j].c3[mip][self.pos[j]][it] * z_pos
            + self.oscs[j].c2[mip][self.pos[j]][it])
            * z_pos
            + self.oscs[j].c1[mip][self.pos[j]][it])
            * z_pos
            + self.oscs[j].c0[mip][self.pos[j]][it];
        //self.interpolated[i] = temp;
        self.voice[i].unison_its[j][0] += x;
        //
        if self.voice[i].unison_its[j][0] > (self.oscs[j].mips[mip][0].len()) as f32 {
            //loop back around zero.
            self.voice[i].unison_its[j][0] -= (self.oscs[j].mips[mip][0].len()) as f32;
        }
        return temp;
    }
    //Convolves a single sample, based on the sample buffer
    pub(crate) fn single_convolve(&self, p_coeffs: &Vec<f32>) -> f32 {
        let mut convolved: f32;
        convolved = 0.;
        //convolved.resize(p_in.len() + p_coeffs.len(), 0.);
        //let mut temp = self.interp_buffer.to_vec();
        //temp.resize(new_len, 0.);
        //n should be the length of p_in + length of p_coeffs
        //this k value should skip the group delay?
        let k = p_coeffs.len();
        for i in 0..k
        //  position in coefficients array
        {
            //if k >= i
            //{
            convolved += p_coeffs[i] * self.interp_buffer[k - i];
            //}
        }
        return convolved;
    }
}
impl Default for Voiceset {
    fn default() -> Voiceset {
        let a = Voiceset {
            filter: vec![filter::DecentFilter::default(); 8],
            oscs: vec![Default::default(); 2], g_oscs: vec![Default::default(); 2],
            vol: vec![1.; 2],
            detune: vec![1.; 2],
            voice: vec![Voice::default(); 8],
            interp_buffer: VecDeque::with_capacity(200),
            pos: vec![0; 2],
            octave: vec![0; 2],
            vol_env : modmatrix::Env {decay_time : 0, sustain : 1., attack_slope : 1.6,..Default::default()},
            mod_env : Default::default(),
        };
        return a;
    }
}
#[derive(Clone)]
pub struct Voice {
    free: bool,
    //every voice can share the same interpolator
    //pub(crate) oscs : &'a Interp,
    unison_its: Vec<Vec<f32>>,
    pub ratio: f32,
    pub(crate) current_mip: usize,
    //pos gives the current wave
    pub note: Option<u8>,
    pub time : usize,
    //the note parameter can allow us to have note offsets for octave and semitone switches
}

#[allow(dead_code)]
impl Voice {
    pub fn reset_its(&mut self) {
        //reset iterators. Value they get set to could be changed to change phase,
        //or made random for analog-style random phase
        //https://rust-lang-nursery.github.io/rust-cookbook/algorithms/randomness.html
        self.unison_its[0][0] = 0.;
        self.unison_its[1][0] = 0.;
    }
    pub fn is_free(&self) -> bool {
        return self.free;
    }
    pub fn use_voice(&mut self, note: u8) {
        self.free = false;
        self.note = Some(note);
        self.time = 0;
        //possibly call prep_buffer here?
    }
    pub fn free_voice(&mut self) {
        //if self.note == note {
        self.free = true;
        //}
    }
}
impl Default for Voice {
    fn default() -> Voice {
        let mut a = Voice {
            free: true,
            unison_its: Vec::with_capacity(7),
            current_mip: 0,
            ratio: 1.,
            note: None,
            time : 0,
        };
        a.unison_its = vec![vec![0.; 7]; 2];
        return a;
    }
}