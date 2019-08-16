use std::f32::consts::PI;
use std::sync::atomic::{AtomicUsize};
use std::sync::Arc;
use vst::util::AtomicFloat;
#[derive(PartialEq)]
#[allow(dead_code)]
/*
    Rethink modboy's use so it directly changes parameters? Potentially avoids some conditionals
*/
enum Method {
    Linear,  // linear solution
    Pivotal, // Mystran's "cheap" method, using x=0 as pivot
}
//this is a 4-pole filter with resonance, which is why there's 4 states and vouts
#[derive(Clone)]
pub struct LadderFilter {
    // Store a handle to the plugin's parameter object.
    pub params: Arc<LadderParameters>,
    // the output of the different filter stages
    pub vout: [f32; 4],
    // s is the "state" parameter. In an IIR it would be the last value from the filter
    // In this we find it by trapezoidal integration to avoid the unit delay
    s: [f32; 4],
}

//default values for parameters
impl Default for LadderFilter {
    fn default() -> LadderFilter {
        LadderFilter {
            vout: [0f32; 4],
            s: [0f32; 4],
            params: Arc::new(LadderParameters::default()),
        }
    }
}
pub struct LadderParameters {
    // the "cutoff" parameter. Determines how heavy filtering is
    pub cutoff: AtomicFloat,
    pub g: AtomicFloat,
    // needed to calculate cutoff.
    sample_rate: AtomicFloat,
    // makes a peak at cutoff
    pub res: AtomicFloat,
    // used to choose where we want our output to be
    pub poles: AtomicUsize,
    // pole_value is just to be able to use get_parameter on poles
    pub pole_value: AtomicFloat,
    // a drive parameter. Just used to increase the volume, which results in heavier distortion
    pub drive: AtomicFloat,
}

impl Default for LadderParameters {
    fn default() -> LadderParameters {
        LadderParameters {
            cutoff: AtomicFloat::new(1000.),
            res: AtomicFloat::new(2.),
            poles: AtomicUsize::new(3),
            pole_value: AtomicFloat::new(1.),
            drive: AtomicFloat::new(0.),
            sample_rate: AtomicFloat::new(44100.),
            g: AtomicFloat::new(0.07135868087),
        }
    }
}
// member methods for the struct
impl LadderFilter {
    // the state needs to be updated after each process. Found by trapezoidal integration
    fn update_state(&mut self) {
        self.s[0] = 2. * self.vout[0] - self.s[0];
        self.s[1] = 2. * self.vout[1] - self.s[1];
        self.s[2] = 2. * self.vout[2] - self.s[2];
        self.s[3] = 2. * self.vout[3] - self.s[3];
    }
    // performs a complete filter process (mystran's method)
    pub fn tick_pivotal(&mut self, input: f32, modboy: Option<f32>, amount: f32) {
        if self.params.drive.get() > 0. {
            self.run_ladder_nonlinear(input * (self.params.drive.get() + 0.7), modboy, amount);
        } else {
            //
            self.run_ladder_linear(input, modboy, amount);
        }
        self.update_state();
    }

    // nonlinear ladder filter function with distortion.
    fn run_ladder_nonlinear(&mut self, input: f32, modboy: Option<f32>, amount: f32) {
        let g = if modboy == None {
            self.params.g.get()
        } else {
            //let cutoff = self.cutoff * ((1.8f32.powf(10. * modboy.unwrap() - 10.)));
            //(3.1415 * self.cutoff / (self.sample_rate)).tan();
            //consider doing min-max on cutoff instead to simplify what's happening
            (PI * (self.params.cutoff.get() + (20000. * (modboy.unwrap() - 0.5) * amount))
                / (self.params.sample_rate.get()))
            .tan()
            .min(6.787)
            .max(0.002324)
        };
        let mut a = [1f32; 5];
        let base = [input, self.s[0], self.s[1], self.s[2], self.s[3]];
        // a[n] is the fixed-pivot approximation for tanh()
        for n in 0..base.len() {
            if base[n] != 0. {
                a[n] = base[n].tanh() / base[n];
            } else {
                a[n] = 1.;
            }
        }
        // denominators of solutions of individual stages. Simplifies the math a bit
        let g0 = 1. / (1. + g * a[1]);
        let g1 = 1. / (1. + g * a[2]);
        let g2 = 1. / (1. + g * a[3]);
        let g3 = 1. / (1. + g * a[4]);
        //  these are just factored out of the feedback solution. Makes the math way easier to read
        let f3 = g * a[3] * g3;
        let f2 = g * a[2] * g2 * f3;
        let f1 = g * a[1] * g1 * f2;
        let f0 = g * g0 * f1;
        // outputs a 24db filter
        self.vout[3] = (f0 * input * a[0]
            + f1 * g0 * self.s[0]
            + f2 * g1 * self.s[1]
            + f3 * g2 * self.s[2]
            + g3 * self.s[3])
            / (f0 * self.params.res.get() * a[3] + 1.);
        // since we know the feedback, we can solve the remaining outputs:
        self.vout[0] = g0
            * (g * a[1] * (input * a[0] - self.params.res.get() * a[3] * self.vout[3]) + self.s[0]);
        self.vout[1] = g1 * (g * a[2] * self.vout[0] + self.s[1]);
        self.vout[2] = g2 * (g * a[3] * self.vout[1] + self.s[2]);
    }
    // linear version without distortion
    pub fn run_ladder_linear(&mut self, input: f32, modboy: Option<f32>, amount: f32) {
        let g = if modboy == None {
            self.params.g.get()
        } else {
            //let cutoff = self.cutoff * ((1.8f32.powf(10. * modboy.unwrap() - 10.)));
            //(3.1415 * self.cutoff / (self.sample_rate)).tan();
            //consider doing min-max on cutoff instead to simplify what's happening
            (PI * (self.params.cutoff.get() + (20000. * (modboy.unwrap() - 0.5) * amount))
                / (self.params.sample_rate.get()))
            .tan()
            .min(6.787)
            .max(0.002324)
        };
        // denominators of solutions of individual stages. Simplifies the math a bit
        let g0 = 1. / (1. + g);
        let g1 = g * g0 * g0;
        let g2 = g * g1 * g0;
        let g3 = g * g2 * g0;
        // outputs a 24db filter
        self.vout[3] =
            (g3 * g * input + g0 * self.s[3] + g1 * self.s[2] + g2 * self.s[1] + g3 * self.s[0])
                / (g3 * g * self.params.res.get() + 1.);
        // since we know the feedback, we can solve the remaining outputs:
        self.vout[0] = g0 * (g * (input - self.params.res.get() * self.vout[3]) + self.s[0]);
        self.vout[1] = g0 * (g * self.vout[0] + self.s[1]);
        self.vout[2] = g0 * (g * self.vout[1] + self.s[2]);
    }
}
impl LadderParameters {
    pub fn set_cutoff(&self, value: f32) {
        // cutoff formula gives us a natural feeling cutoff knob that spends more time in the low frequencies
        self.cutoff.set(20000. * (1.8f32.powf(10. * value - 10.)));
        // bilinear transformation for g gives us a very accurate cutoff
        self.g
            .set((PI * self.cutoff.get() / (self.sample_rate.get())).tan());
    }
    // returns the value used to set cutoff. for get_parameter function
    pub fn get_cutoff(&self) -> f32 {
        1. + 0.1701297528 * (0.00005 * self.cutoff.get()).ln()
    }
    // pub fn set_poles(&self, value: f32) {
    //     self.pole_value.set(value);
    //     self.poles
    //         .store(((value * 3.).round()) as usize, Ordering::Relaxed);
    // }
}
// impl PluginParameters for LadderParameters {
//     // get_parameter has to return the value used in set_parameter
//     fn get_parameter(&self, index: i32) -> f32 {
//         match index {
//             0 => self.get_cutoff(),
//             1 => self.res.get() / 4.,
//             2 => self.pole_value.get(),
//             3 => self.drive.get() / 5.,
//             _ => 0.0,
//         }
//     }
//     fn set_parameter(&self, index: i32, value: f32) {
//         match index {
//             0 => self.set_cutoff(value),
//             1 => self.res.set(value * 4.),
//             2 => self.set_poles(value),
//             3 => self.drive.set(value * 5.),
//             _ => (),
//         }
//     }

//     fn get_parameter_name(&self, index: i32) -> String {
//         match index {
//             0 => "cutoff".to_string(),
//             1 => "resonance".to_string(),
//             2 => "filter order".to_string(),
//             3 => "drive".to_string(),
//             _ => "".to_string(),
//         }
//     }
//     fn get_parameter_label(&self, index: i32) -> String {
//         match index {
//             0 => "Hz".to_string(),
//             1 => "%".to_string(),
//             2 => "poles".to_string(),
//             3 => "%".to_string(),
//             _ => "".to_string(),
//         }
//     }
//     // This is what will display underneath our control.  We can
//     // format it into a string that makes the most sense.
//     fn get_parameter_text(&self, index: i32) -> String {
//         match index {
//             0 => format!("{:.0}", self.cutoff.get()),
//             1 => format!("{:.3}", self.res.get()),
//             2 => format!("{}", self.poles.load(Ordering::Relaxed) + 1),
//             3 => format!("{:.3}", self.drive.get()),
//             _ => format!(""),
//         }
//     }
// }
