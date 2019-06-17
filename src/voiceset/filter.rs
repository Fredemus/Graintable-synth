#[allow(dead_code)]
#[derive(PartialEq)]
enum AnalyticalMethod {
    Linear,  // linear solution
    Pivotal, // Mystran's "cheap" method, using x=0 as pivot
}

//this is a 4-pole filter with resonance, which is why there's 4 states and vouts
#[derive(Clone)]
pub struct DecentFilter {
    //stands in as the output of the filter, since it needs to be edited a bunch of places
    pub vout: [f32; 4],
    //s is the "state" parameter. it's normally the last value from the filter (buf0 in the last filter)
    //found by trapezoidal integration in a zero-delay feedback filter
    s: [f32; 4],
    //the "cutoff" parameter. Determines how heavy filtering is
    pub g: f32,
    //needed to calculate cutoff. Should get it from the host instead of just setting it to 44.1k
    pub sample_rate: f32,
    //where the filtering starts
    pub cutoff: f32,
    //makes a peak at cutoff
    pub res: f32,
    //used to choose if we want it to output 1 or 2 order filtering
    pub poles: usize,
    pub oversample: usize,
    pub drive: f32,
}
//member methods for the struct
impl DecentFilter {
    pub fn set_cutoff(&mut self, value: f32) {
        self.cutoff = 20000. * (1.8f32.powf(10. * value - 10.)).min(0.999); //does cutoff formula make sense?
        self.g = (3.1415 * self.cutoff / (self.sample_rate)).tan();
    }
    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
    }
    //the state needs to be updated after each process.
    fn update_state(&mut self) {
        self.s[0] = 2. * self.vout[0] - self.s[0];
        //the s[1] formula should be valid? found by trapezoidal integration?
        self.s[1] = 2. * self.vout[1] - self.s[1];
        self.s[2] = 2. * self.vout[2] - self.s[2];
        self.s[3] = 2. * self.vout[3] - self.s[3];
    }
    //used for getting the starting point for estimating how to compute the filtering
    // fn get_initial_estimate(&mut self, filter_order: usize) -> f32 {
    //     return self.s[filter_order];
    // }

    //performs a complete filter process (mystran's method)
    pub fn tick_pivotal(&mut self, input: f32, modboy : Option<f32>) {
        //let tanh_input = input.tanh();
        if self.drive > 0. {
            self.run_moog_nonlinear(input * (self.drive + 0.7), AnalyticalMethod::Pivotal, modboy);
        } else {
            self.run_moog_nonlinear(input, AnalyticalMethod::Linear, modboy); //linear should have clipping on resonance, i think?
        }
        self.update_state();
    }
    fn _tick_simple(&mut self, input: f32) {
        self.run_moog_simple(input * (self.drive + 0.7));
        self.update_state();
    }
    //instead of proper nonlinearities, this just has soft-clipping on the input
    fn run_moog_simple(&mut self, input: f32) {
        let x = input.tanh();
        //denominators of solutions of individual stages. Simplifies the math a bit
            let g0 = 1. / (1. + self.g);
            let g1 = self.g * g0 * g0;
            let g2 = self.g * g1 * g0;
            let g3 = self.g * g2 * g0;
            //outputs a 24db filter
            self.vout[3] = (g3 * self.g * x
                + g0 * self.s[3]
                + g1 * self.s[2]
                + g2 * self.s[1]
                + g3 * self.s[0])
                / (g3 * self.g * self.res + 1.);
            //since we know the feedback, we can solve the remaining outputs:
            self.vout[0] = g0 * (self.g * (x - self.res * self.vout[3]) + self.s[0]);
            self.vout[1] = g0 * (self.g * self.vout[0] + self.s[1]);
            self.vout[2] = g0 * (self.g * self.vout[1] + self.s[2]);
    }

    //nonlinear ladder filter function.  
    fn run_moog_nonlinear(&mut self, input: f32, method: AnalyticalMethod, modboy : Option<f32>) {
        let g : f32;
        if modboy == None {
            g = self.g;
        }
        else {
            g = (3.1415 * (self.cutoff * modboy.unwrap()) / (self.sample_rate)).tan();
        }
        let mut a = [1f32; 5];
        //version with drive
        if method == AnalyticalMethod::Pivotal {
            let base = [
                0.,//self.res * self.s[3],
                self.s[0],
                self.s[1],
                self.s[2],
                self.s[3],
            ];
            for n in 0..base.len() {
                if base[n] != 0. {
                    a[n] = base[n].tanh() / base[n];
                } else {
                    a[n] = 1.;
                }
            }
            //denominators of solutions of individual stages. Simplifies the math a bit
            let g0 = 1. / (1. + g * a[1]); let g1 = 1. / (1. + g * a[2]);
            let g2 = 1. / (1. + g * a[3]); let g3 = 1. / (1. + g * a[4]);
            // these are just factored out of the feedback solution. Makes the math way easier to read
            let f3 = g * a[3] * g3; let f2 = g * a[2] * g2 * f3;
            let f1 = g * a[1] * g1 * f2; let f0 = g * g0 * f1;
            //outputs a 24db filter
            self.vout[3] = (f0 * input + f1 * g0 * self.s[0]
                + f2 * g1 * self.s[1]
                + f3 * g2 * self.s[2]
                + g3 * self.s[3])
                / (f0 * self.res * a[3] + 1.);

            //since we know the feedback, we can solve the remaining outputs:
            self.vout[0] =
                g0 * (g * a[1] * (input - self.res * a[3] * self.vout[3]) + self.s[0]);
            self.vout[1] = g1 * (g * a[2] * self.vout[0] + self.s[1]);
            self.vout[2] = g2 * (g * a[3] * self.vout[1] + self.s[2]);
        }
        //linear version without.
        else {
            //denominators of solutions of individual stages. Simplifies the math a bit
            let g0 = 1. / (1. + g);
            let g1 = g * g0 * g0;
            let g2 = g * g1 * g0;
            let g3 = g * g2 * g0;
            //outputs a 24db filter
            self.vout[3] = (g3 * g * input
                + g0 * self.s[3]
                + g1 * self.s[2]
                + g2 * self.s[1]
                + g3 * self.s[0])
                / (g3 * g * self.res + 1.);
            //since we know the feedback, we can solve the remaining outputs:
            self.vout[0] = g0 * (g * (input - self.res * self.vout[3]) + self.s[0]);
            self.vout[1] = g0 * (g * self.vout[0] + self.s[1]);
            self.vout[2] = g0 * (g * self.vout[1] + self.s[2]);
        }
    }
}

//default values for parameters
impl Default for DecentFilter {
    fn default() -> DecentFilter {
        DecentFilter {
            vout: [0f32; 4],
            s: [0f32; 4],
            sample_rate: 88200.,
            cutoff: 1000.,
            res: 2.0,
            g: 0.07135868087,
            poles: 3,
            oversample: 1,
            drive: 0.,
        }
    }
}
//can be pilfered for gets and sets of parameters
// impl Plugin for DecentFilter
// {

//     fn get_info(&self) -> Info
//     {
//         Info  {
//             name: "ZeroDelayFilter".to_string(),
//             unique_id: 9263,
//             inputs: 1,
//             outputs: 1,
//             category: Category::Effect,
//             parameters: 4,
//             ..Default::default()
//         }
//     }
//     fn get_parameter(&self, index: i32) -> f32 {
//     match index {
//         0 => self.cutoff,
//         1 => self.res,
//         2 => (self.poles) as f32 + 1.,
//         3 => self.oversample as f32,
//         _ => 0.0,
//         }
//     }
//     fn set_parameter(&mut self, index: i32, value: f32) {
//         match index {
//             0 => self.cutoff = 20000. * (1.8f32.powf(10. * value - 10.)).min(0.999),
//             //self.g = value * 10.,
//             1 => self.res = value * 4.4,
//             2 => self.poles = ((value * 3.).round()) as usize,
//             3 => self.oversample = ((value * 2.).round()).exp2() as usize,
//             _ => (),
//         }
//         //the formula for g seems to be accurate, but it could have something to do with the cutoff problem
//         self.g = ( 3.1415 * self.cutoff / (self.sample_rate * self.oversample as f32)).tan();

//     }

//     fn get_parameter_name(&self, index: i32) -> String {
//         match index {
//             0 => "cutoff".to_string(),
//             1 => "res".to_string(),
//             2 => "filter order".to_string(),
//             3 => "oversampling".to_string(),
//             _ => "".to_string(),
//         }
//     }
//     fn get_parameter_label(&self, index: i32) -> String {
//         match index {
//             0 => "Hz".to_string(),
//             1 => "%".to_string(),
//             2 => "poles".to_string(),
//             3 => "times".to_string(),
//             _ => "".to_string(),
//         }
//     }
//     fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
//         if self.oversample > 1 {
//              for (input_buffer, output_buffer) in buffer.zip() {
//                 for (input_sample, output_sample) in input_buffer.iter().zip(output_buffer) {
//                     //self.tick_newton_raphson(*input_sample);
//                     //oversampling process
//                     //we oversample with a zero-stuffing process. Leaves original signal untouched, but creates
//                     //a mirror image at twice the frequency

//                     self.upsample_array[0] = *input_sample;
//                     for n in 1..self.oversample {
//                     self.upsample_array[n] = 0.;
//                     }
//                     //after upsample process, we should use a steep lowpass to get rid of the mirror image
//                     //omitted for now, since our process is a lowpass anyway. Might give trouble at high frequencies
//                     for n in 0..self.oversample {

//                         //self.tick_newton_raphson(self.upsample_array[n]);
//                         self.tick_pivotal(self.upsample_array[n]);
//                         //downsampling
//                         if n == 0 {
//                             //self.oversample used as gain factor, since zero-stuffing reduces volume
//                             *output_sample = self.vout[self.poles] * (self.oversample as f32);
//                         }
//                     }
//                     //self.tick_pivotal(*input_sample);
//                 }
//             }
//         }
//         else {
//             for (input_buffer, output_buffer) in buffer.zip() {
//                 for (input_sample, output_sample) in input_buffer.iter().zip(output_buffer) {
//                     self.tick_pivotal(*input_sample);
//                     //self.tick_linear(*input_sample);
//                     //self.tick_newton_raphson(*input_sample);
//                     *output_sample = self.vout[self.poles];
//                 }
//             }
//         }
//     }
// }