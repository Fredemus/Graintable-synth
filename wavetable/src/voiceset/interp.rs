extern crate hound;
use std::sync::Arc;
use crate::util::{AtomicF32, AtomicUsize};
pub fn mip_offset(mip: usize, len: usize) -> usize {
    let amount = match mip {
        0 => 0.,
        1 => 1.,
        2 => 1.5,
        3 => 1.75,
        4 => 1.875,
        5 => 1.9375,
        6 => 1.96875,
        7 => 1.984375,
        8 => 1.9921875,
        9 => 1.99609375,
        _ => 0.,
    };
    (len as f32 * amount) as usize
}
pub struct GrainParams {
    pub grain_size: AtomicF32,
    pub pos: AtomicF32,
    pub len: AtomicUsize,
}
impl Default for GrainParams {
    fn default() -> GrainParams {
        GrainParams {
            grain_size: AtomicF32::new(4096.),
            pos: AtomicF32::new(0.),
            len: AtomicUsize::new(0),
        }
    }
}
#[derive(Clone)]
pub struct GrainTable {
    pub params: Arc<GrainParams>,
    pub(crate) source_y: Vec<f32>,
    pub(crate) wave_number: usize,
    pub wave_len: usize,
    pub(crate) amt_oversample: usize,
    pub c0: Vec<f32>,
    pub c1: Vec<f32>,
    pub c2: Vec<f32>,
    pub c3: Vec<f32>,
    //pub it: usize,
    pub(crate) upsample_fir: Vec<f32>,
    pub(crate) downsample_fir: Vec<f32>,
    pub mips: Vec<f32>,
    mip_levels: usize,
}
#[allow(dead_code)]
impl GrainTable {
    pub fn change_table(&mut self, path: String) {
        let mut reader = hound::WavReader::open(path).unwrap();
        self.source_y = reader.samples().collect::<Result<Vec<_>, _>>().unwrap();
        self.oversample(2);
        self.mip_map();
        self.optimal_coeffs();
        // number of waves in table needs to be passed to params to avoid crashes 
    }

    pub fn mip_len(&self, mip: usize) -> usize {
        (self.source_y.len() as f32 / 2f32.powi(mip as i32)) as usize
    }
    // pub fn set_pos(&mut self, value : f32) -> usize {
    //     ((value * (self.wave_number - 1) as f32).round()) as usize
    // }
    pub(crate) fn oversample(&mut self, ratio: usize) {
        self.amt_oversample = ratio;
        //resize sourcey to fit the new length
        self.source_y.resize(self.source_y.len() * ratio, 0.);
        let mut temp = vec![0.];
        temp.resize(self.source_y.len(), 0.);
        //fills temp with an oversampled version of current slice
        for j in 0..(self.source_y.len()) {
            if j % ratio == 0 {
                temp[j] = self.source_y[j / ratio];
            } else {
                temp[j] = 0.;
            }
        }
        self.source_y = temp.to_vec();
        //static_convolve zero-stuffed vector with coefficients (sinc) of a fir, to remove mirror images above new_Fs/4
        //upsample_fir could be turned into a polyphase implementation, to halve number of clock cycles needed
        self.source_y = self.static_convolve(&self.upsample_fir, &self.source_y);
    }
    pub(crate) fn downsample_2x(&self, signal: &Vec<f32>) -> Vec<f32> {
        //first we filter the signal to downsample 2x
        let temp = self.static_convolve(&self.downsample_fir, &signal);
        let mut output = vec![0.];
        output.resize(temp.len() / 2, 0.);
        for j in 0..(signal.len() / 2) {
            output[j] = temp[j * 2];
        }
        output
    }

    pub(crate) fn mip_map(&mut self) {
        self.mips.resize(self.source_y.len(), 0.);

        //fill first layer with self.source_y
        self.mips = self.source_y.clone();
        //fills the mip_levels with continually more downsampled vectors
        let mut temp = self.downsample_2x(&self.source_y);
        for _i in 1..self.mip_levels {
            self.mips.append(&mut temp.clone());
            temp = self.downsample_2x(&temp);
        }
    }

    pub(crate) fn optimal_coeffs(&mut self) {
        self.params.len.set(self.source_y.len());
        //let len = self.params.len;
        let len = self.source_y.len();
        /*
        // Optimal 2x (4-point, 3rd-order) (z-form)
        float z = x - 1/2.0;
        float even1 = y[1]+y[0], odd1 = y[1]-y[0];
        float even2 = y[2]+y[-1], odd2 = y[2]-y[-1];
        float c0 = even1*0.45868970870461956 + even2*0.04131401926395584;
        float c1 = odd1*0.48068024766578432 + odd2*0.17577925564495955;
        float c2 = even1*-0.246185007019907091 + even2*0.24614027139700284;
        float c3 = odd1*-0.36030925263849456 + odd2*0.10174985775982505;
        */
        let mut even1: f32;
        let mut even2: f32;
        let mut odd1: f32;
        let mut odd2: f32;

        self.c0.resize(self.source_y.len() * 2, 0.);
        self.c1.resize(self.source_y.len() * 2, 0.);
        self.c2.resize(self.source_y.len() * 2, 0.);
        self.c3.resize(self.source_y.len() * 2, 0.);
        // for i in 0..self.mip_levels {
        //     self.c0[i].resize(self.mip_len(i), 0.);
        //     self.c1[i].resize(self.mip_len(i), 0.);
        //     self.c2[i].resize(self.mip_len(i), 0.);
        //     self.c3[i].resize(self.mip_len(i), 0.);
        // }
        for n in 0..self.mip_levels {
            //n represent mip-map levels
            for j in 1..self.mip_len(n) - 2 {
                even1 =
                    self.mips[mip_offset(n, len) + j + 1] + self.mips[mip_offset(n, len) + j + 0];
                odd1 =
                    self.mips[mip_offset(n, len) + j + 1] - self.mips[mip_offset(n, len) + j + 0];
                even2 =
                    self.mips[mip_offset(n, len) + j + 2] + self.mips[mip_offset(n, len) + j - 1];
                odd2 =
                    self.mips[mip_offset(n, len) + j + 2] - self.mips[mip_offset(n, len) + j - 1];
                self.c0[mip_offset(n, len) + j] =
                    even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                self.c1[mip_offset(n, len) + j] =
                    odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                self.c2[mip_offset(n, len) + j] =
                    even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                self.c3[mip_offset(n, len) + j] =
                    odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
            }
        }
        //makes sure the start of waveforms are handled properly
        for n in 0..self.mip_levels {
            let j = self.mip_len(n);
            even1 = self.mips[mip_offset(n, len) + 1] + self.mips[mip_offset(n, len) + 0];
            odd1 = self.mips[mip_offset(n, len) + 1] - self.mips[mip_offset(n, len) + 0];
            even2 = self.mips[mip_offset(n, len) + 2] + self.mips[mip_offset(n, len) + j - 1];
            odd2 = self.mips[mip_offset(n, len) + 2] - self.mips[mip_offset(n, len) + j - 1];
            self.c0[mip_offset(n, len)] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
            self.c1[mip_offset(n, len)] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
            self.c2[mip_offset(n, len)] =
                even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
            self.c3[mip_offset(n, len)] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
        }
        //makes sure the end of waveforms are handled properly
        for n in 0..self.mip_levels {
            let j = self.mip_len(n);
            even1 = self.mips[mip_offset(n, len) + j - 1] + self.mips[mip_offset(n, len) + j - 2];
            odd1 = self.mips[mip_offset(n, len) + j - 1] - self.mips[mip_offset(n, len) + j - 2];
            even2 = self.mips[mip_offset(n, len)] + self.mips[mip_offset(n, len) + j - 3];
            odd2 = self.mips[mip_offset(n, len)] - self.mips[mip_offset(n, len) + j - 3];
            self.c0[mip_offset(n, len) + j - 2] =
                even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
            self.c1[mip_offset(n, len) + j - 2] =
                odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
            self.c2[mip_offset(n, len) + j - 2] =
                even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
            self.c3[mip_offset(n, len) + j - 2] =
                odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;

            even1 = self.mips[mip_offset(n, len) + 0] + self.mips[mip_offset(n, len) + j - 1];
            odd1 = self.mips[mip_offset(n, len) + 0] - self.mips[mip_offset(n, len) + j - 1];
            even2 = self.mips[mip_offset(n, len) + 1] + self.mips[mip_offset(n, len) + j - 2];
            odd2 = self.mips[mip_offset(n, len) + 1] - self.mips[mip_offset(n, len) + j - 2];
            self.c0[mip_offset(n, len) + j - 1] =
                even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
            self.c1[mip_offset(n, len) + j - 1] =
                odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
            self.c2[mip_offset(n, len) + j - 1] =
                even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
            self.c3[mip_offset(n, len) + j - 1] =
                odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
        }
    }

    pub(crate) fn static_convolve(&self, p_coeffs: &Vec<f32>, p_in: &Vec<f32>) -> Vec<f32> {
        //possibly more efficient convolution https://stackoverflow.com/questions/8424170/1d-linear-convolution-in-ansi-c-code
        //convolution could be significantly sped up by doing it in the frequency domain. from O(n^2) to O(n*log(n))
        let mut convolved: Vec<f32>;
        let new_len = p_in.len() + (p_coeffs.len() - 1) / 2;
        convolved = vec![0.; p_in.len() + p_coeffs.len()];
        //convolved.resize(p_in.len() + p_coeffs.len(), 0.);
        let mut temp = p_in.to_vec();
        temp.resize(new_len, 0.);
        //n should be the length of p_in + length of p_coeffs
        for k in 0..(new_len)
        //  position in output
        {
            for i in 0..p_coeffs.len()
            //  position in coefficients array
            {
                if k >= i {
                    convolved[k] += p_coeffs[i] * temp[k - i];
                }
            }
        }
        //trimming the result
        //remove initial group delay by taking number of coefficients - 1 / 2. Only works for odd number of coefficients
        for _i in 0..(p_coeffs.len() - 1) / 2 {
            convolved.remove(0); //maybe use drain on an iterator instead?
        }
        //trims unnecessary samples at the end
        convolved.truncate(p_in.len());
        return convolved;
    }
}
impl Default for GrainTable {
    fn default() -> GrainTable {
        GrainTable {
            params: Arc::new(GrainParams::default()),
            source_y: Vec::with_capacity(4096 * 256),
            mips: Vec::with_capacity(10),
            mip_levels: 7,
            wave_number: 0,
            amt_oversample: 1,
            wave_len: 2048,
            //it: 0,
            //default capacity should take oversampling into account
            //capacity probably needs only to be the number of mips
            c0: Vec::with_capacity(8),
            c1: Vec::with_capacity(8),
            c2: Vec::with_capacity(8),
            c3: Vec::with_capacity(8),

            upsample_fir: vec![
                5.807e-05,
                0.00015957,
                0.00017629,
                4.1774e-06,
                -0.00021049,
                -0.0001965,
                2.3485e-05,
                9.5114e-05,
                -0.00011663,
                -0.00024653,
                -1.8106e-05,
                0.00021017,
                3.0079e-06,
                -0.00032069,
                -0.00014625,
                0.00029734,
                0.00020506,
                -0.00034762,
                -0.00036956,
                0.00029091,
                0.0004813,
                -0.00025542,
                -0.00065389,
                0.0001256,
                0.00077988,
                1.7544e-05,
                -0.0009229,
                -0.00024499,
                0.0010065,
                0.00050125,
                -0.0010623,
                -0.00082416,
                0.0010349,
                0.0011698,
                -0.00093752,
                -0.0015501,
                0.00073146,
                0.001924,
                -0.00042358,
                -0.0022835,
                -8.6855e-06,
                0.0025865,
                0.00055521,
                -0.0028116,
                -0.0012206,
                0.0029163,
                0.0019844,
                -0.0028726,
                -0.0028315,
                0.0026421,
                0.003727,
                -0.0021982,
                -0.0046357,
                0.0015123,
                0.0055074,
                -0.00056789,
                -0.0062889,
                -0.00064776,
                0.0069164,
                0.0021342,
                -0.0073232,
                -0.0038835,
                0.0074351,
                0.005874,
                -0.0071756,
                -0.0080748,
                0.0064621,
                0.010443,
                -0.0052066,
                -0.012928,
                0.0033099,
                0.015469,
                -0.00065173,
                -0.018001,
                -0.0029281,
                0.020454,
                0.0076684,
                -0.022758,
                -0.013972,
                0.024844,
                0.0226,
                -0.026649,
                -0.035204,
                0.028117,
                0.056141,
                -0.029201,
                -0.10152,
                0.029865,
                0.31677,
                0.46991,
                0.31677,
                0.029865,
                -0.10152,
                -0.029201,
                0.056141,
                0.028117,
                -0.035204,
                -0.026649,
                0.0226,
                0.024844,
                -0.013972,
                -0.022758,
                0.0076684,
                0.020454,
                -0.0029281,
                -0.018001,
                -0.00065173,
                0.015469,
                0.0033099,
                -0.012928,
                -0.0052066,
                0.010443,
                0.0064621,
                -0.0080748,
                -0.0071756,
                0.005874,
                0.0074351,
                -0.0038835,
                -0.0073232,
                0.0021342,
                0.0069164,
                -0.00064776,
                -0.0062889,
                -0.00056789,
                0.0055074,
                0.0015123,
                -0.0046357,
                -0.0021982,
                0.003727,
                0.0026421,
                -0.0028315,
                -0.0028726,
                0.0019844,
                0.0029163,
                -0.0012206,
                -0.0028116,
                0.00055521,
                0.0025865,
                -8.6855e-06,
                -0.0022835,
                -0.00042358,
                0.001924,
                0.00073146,
                -0.0015501,
                -0.00093752,
                0.0011698,
                0.0010349,
                -0.00082416,
                -0.0010623,
                0.00050125,
                0.0010065,
                -0.00024499,
                -0.0009229,
                1.7544e-05,
                0.00077988,
                0.0001256,
                -0.00065389,
                -0.00025542,
                0.0004813,
                0.00029091,
                -0.00036956,
                -0.00034762,
                0.00020506,
                0.00029734,
                -0.00014625,
                -0.00032069,
                3.0079e-06,
                0.00021017,
                -1.8106e-05,
                -0.00024653,
                -0.00011663,
                9.5114e-05,
                2.3485e-05,
                -0.0001965,
                -0.00021049,
                4.1774e-06,
                0.00017629,
                0.00015957,
                5.807e-05,
            ],
            downsample_fir: vec![
                6.6501e-05,
                0.00016925,
                0.00013554,
                -9.6855e-05,
                -0.00025644,
                -0.00011532,
                6.7101e-05,
                -4.2646e-05,
                -0.00020891,
                -5.5997e-05,
                0.00014325,
                -3.4041e-05,
                -0.0002427,
                -5.113e-06,
                0.00023407,
                -5.8992e-05,
                -0.00031185,
                6.2543e-05,
                0.0003345,
                -0.00012665,
                -0.00040184,
                0.00016754,
                0.00043956,
                -0.00024865,
                -0.00049771,
                0.0003276,
                0.00053631,
                -0.00043767,
                -0.00057921,
                0.0005572,
                0.0006033,
                -0.00070362,
                -0.00061865,
                0.00086496,
                0.00061014,
                -0.00105,
                -0.00058064,
                0.0012506,
                0.00051885,
                -0.0014698,
                -0.00042359,
                0.0017007,
                0.00028602,
                -0.0019419,
                -0.00010262,
                0.0021861,
                -0.00013333,
                -0.0024277,
                0.0004261,
                0.0026582,
                -0.00078058,
                -0.0028689,
                0.0012001,
                0.0030488,
                -0.0016878,
                -0.0031867,
                0.0022447,
                0.0032692,
                -0.0028717,
                -0.0032825,
                0.0035672,
                0.0032112,
                -0.0043288,
                -0.003039,
                0.0051523,
                0.0027483,
                -0.0060318,
                -0.0023202,
                0.0069598,
                0.0017343,
                -0.0079273,
                -0.00096783,
                0.0089238,
                -5.1968e-06,
                -0.0099377,
                0.0012151,
                0.010956,
                -0.0026994,
                -0.011965,
                0.0045069,
                0.012951,
                -0.0067036,
                -0.013899,
                0.0093857,
                0.014795,
                -0.012698,
                -0.015624,
                0.016878,
                0.016373,
                -0.022335,
                -0.01703,
                0.029851,
                0.017583,
                -0.041109,
                -0.018024,
                0.060505,
                0.018345,
                -0.10419,
                -0.01854,
                0.31767,
                0.5186,
                0.31767,
                -0.01854,
                -0.10419,
                0.018345,
                0.060505,
                -0.018024,
                -0.041109,
                0.017583,
                0.029851,
                -0.01703,
                -0.022335,
                0.016373,
                0.016878,
                -0.015624,
                -0.012698,
                0.014795,
                0.0093857,
                -0.013899,
                -0.0067036,
                0.012951,
                0.0045069,
                -0.011965,
                -0.0026994,
                0.010956,
                0.0012151,
                -0.0099377,
                -5.1968e-06,
                0.0089238,
                -0.00096783,
                -0.0079273,
                0.0017343,
                0.0069598,
                -0.0023202,
                -0.0060318,
                0.0027483,
                0.0051523,
                -0.003039,
                -0.0043288,
                0.0032112,
                0.0035672,
                -0.0032825,
                -0.0028717,
                0.0032692,
                0.0022447,
                -0.0031867,
                -0.0016878,
                0.0030488,
                0.0012001,
                -0.0028689,
                -0.00078058,
                0.0026582,
                0.0004261,
                -0.0024277,
                -0.00013333,
                0.0021861,
                -0.00010262,
                -0.0019419,
                0.00028602,
                0.0017007,
                -0.00042359,
                -0.0014698,
                0.00051885,
                0.0012506,
                -0.00058064,
                -0.00105,
                0.00061014,
                0.00086496,
                -0.00061865,
                -0.00070362,
                0.0006033,
                0.0005572,
                -0.00057921,
                -0.00043767,
                0.00053631,
                0.0003276,
                -0.00049771,
                -0.00024865,
                0.00043956,
                0.00016754,
                -0.00040184,
                -0.00012665,
                0.0003345,
                6.2543e-05,
                -0.00031185,
                -5.8992e-05,
                0.00023407,
                -5.113e-06,
                -0.0002427,
                -3.4041e-05,
                0.00014325,
                -5.5997e-05,
                -0.00020891,
                -4.2646e-05,
                6.7101e-05,
                -0.00011532,
                -0.00025644,
                -9.6855e-05,
                0.00013554,
                0.00016925,
                6.6501e-05,
            ],
        }
    }
}

#[derive(Clone)]
pub struct WaveTable {
    pub(crate) source_y: Vec<f32>,
    pub(crate) waveforms: Vec<Vec<f32>>,
    pub(crate) wave_number: usize,
    pub wave_len: usize,
    pub(crate) len: usize,
    pub(crate) amt_oversample: usize,

    pub c0: Vec<Vec<Vec<f32>>>,
    pub c1: Vec<Vec<Vec<f32>>>,
    pub c2: Vec<Vec<Vec<f32>>>,
    pub c3: Vec<Vec<Vec<f32>>>,

    //pub it: usize,

    pub(crate) upsample_fir: Vec<f32>,
    pub(crate) downsample_fir: Vec<f32>,
    pub mips: Vec<Vec<Vec<f32>>>,
    mip_levels: usize,
}
#[allow(dead_code)]
impl WaveTable {
    pub fn change_table(&mut self, path: String) {
        let mut reader = hound::WavReader::open(path).unwrap();
        self.source_y = reader.samples().collect::<Result<Vec<_>, _>>().unwrap();
        self.slice();
        self.oversample(2);
        self.mip_map();
        self.optimal_coeffs();
    }
    pub(crate) fn oversample(&mut self, ratio: usize) {
        self.amt_oversample = ratio;
        //resize slices to fit the new length
        self.wave_len *= ratio;
        for i in 0..self.wave_number {
            self.waveforms[i].resize(self.wave_len, 0.);
        }
        let mut temp = vec![0.];
        temp.resize(self.wave_len, 0.);
        for i in 0..self.wave_number {
            //fills temp with an oversampled version of current slice
            for j in 0..self.wave_len {
                if j % ratio == 0 {
                    temp[j] = self.waveforms[i][j / ratio];
                } else {
                    temp[j] = 0.;
                }
            }
            self.waveforms[i] = temp.to_vec();
        }
        //static_convolve zero-stuffed vector with coefficients (sinc) of a fir, to remove mirror images above new_Fs/4
        //upsample_fir could be turned into a polyphase implementation, to halve number of clock cycles needed
        for i in 0..self.wave_number {
            self.waveforms[i] = self.static_convolve(&self.upsample_fir, &self.waveforms[i]);
        }
    }
    pub(crate) fn downsample_2x(&self, signal: &Vec<f32>) -> Vec<f32> {
        //first we filter the signal to downsample 2x
        let temp = self.static_convolve(&self.downsample_fir, &signal);
        let mut output = vec![0.];
        output.resize(temp.len() / 2, 0.);
        for j in 0..(signal.len() / 2) {
            output[j] = temp[j * 2];
        }
        output
    }

    pub(crate) fn mip_map(&mut self) {
        self.mips.resize(
            self.mip_levels,
            vec![vec![0.; self.wave_len]; self.wave_number],
        );

        //fill first layer with self.waveforms
        for i in 0..self.wave_number {
            self.mips[0][i] = self.waveforms[i].to_vec();
        }
        //fills the mip_levels with continually more downsampled vectors
        for i in 1..self.mip_levels {
            for j in 0..self.wave_number {
                self.mips[i][j] = self.downsample_2x(&self.mips[i - 1][j]);
            }
        }
    }
    //check for off-by-ones at some point. self.len should be fine instead of len_x
    pub(crate) fn hermite_coeffs(&mut self) {
        // self.len = self.source_y.len();
        // let new_wave_len = self.wave_len *self.amt_oversample;
        // /*
        // // 4-point, 3rd-order Hermite (x-form)
        // float c0 = y[0];
        // float c1 = 1/2.0*(y[1]-y[-1]);
        // float c2 = y[-1] - 5/2.0*y[0] + 2*y[1] - 1/2.0*y[2];
        // float c3 = 1/2.0*(y[2]-y[-1]) + 3/2.0*(y[0]-y[1]);
        // return ((c3*x+c2)*x+c1)*x+c0;
        // */
        // self.c0.resize(self.wave_number,vec![vec![0.;new_wave_len]; 5]);
        // self.c1.resize(self.wave_number,vec![vec![0.;new_wave_len]; 5]);
        // self.c2.resize(self.wave_number,vec![vec![0.;new_wave_len]; 5]);
        // self.c3.resize(self.wave_number,vec![vec![0.;new_wave_len]; 5]);
        // //this could easily be optimized away, but oh well
        // for i in 0..self.wave_number {
        //     for j in 0..new_wave_len {
        //         self.c0[i][j] = self.waveforms[i][j];
        //     }
        // }
        // //instead of len_x it should be 0+cyclelength. doesn't seem to be a big problem
        // //self.c1[0] =  1./2.0*(self.source_y[0+1] - self.source_y[len_x]);
        // //self.c2[0] =  self.source_y[len_x] - 5./2.0*self.source_y[0] + 2.*self.source_y[0+1] - 1.0/2.0*self.source_y[0+2];
        // //self.c3[0] =  1./2.0*(self.source_y[0+2]-self.source_y[len_x]) + 3.0/2.0*(self.source_y[0+0]-self.source_y[0+1]);
        // for i in 0..self.wave_number {
        //     for j in 1..new_wave_len - 1 {

        //         self.c1[i][j] =  1./2.0*(self.waveforms[i][j+1] -self.waveforms[i][j-1]);
        //     }
        // }
        // for i in 0..self.wave_number {
        //     for j in 1..new_wave_len - 2 {
        //         self.c2[i][j] =  self.waveforms[i][j-1] - 5./2.0*self.waveforms[i][j] + 2.*self.waveforms[i][j+1] - 1.0/2.0*self.waveforms[i][j+2];
        //     }
        // }
        // for i in 0..self.wave_number {
        //     for j in 1..new_wave_len - 2 {
        //         self.c3[i][j] =  1./2.0*(self.waveforms[i][j+2]-self.waveforms[i][j-1]) + 3.0/2.0*(self.waveforms[i][j+0]-self.waveforms[i][j+1]);
        //     }
        // }
        // //makes sure the start of waveforms are handled properly
        // for i in 0..self.wave_number {
        //     self.c1[i][0] =  (1.0/2.0)*(self.waveforms[i][0+1] - self.waveforms[i][new_wave_len  - 1]);
        //     self.c2[i][0] =  self.waveforms[i][new_wave_len  - 1] - (5./2.0)*self.waveforms[i][0] + 2.*self.waveforms[i][0+1] - (1.0/2.0)*self.waveforms[i][0+2];
        //     self.c3[i][0] =  (1.0/2.0)*(self.waveforms[i][0+2]-self.waveforms[i][new_wave_len  - 1]) + (3.0/2.0)*(self.waveforms[i][0]-self.waveforms[i][0+1]);
        // }
        // //makes sure the end of waveforms are handled properly
        // for i in 0..self.wave_number {
        //     self.c1[i][new_wave_len  - 1] =  1./2.0*(self.waveforms[i][0] - self.waveforms[i][new_wave_len - 2]);
        //     self.c2[i][new_wave_len - 1] =  self.waveforms[i][new_wave_len - 2] - 5./2.0*self.waveforms[i][new_wave_len - 1] + 2.*self.waveforms[i][0] - 1.0/2.0*self.waveforms[i][0+1];
        //     self.c2[i][new_wave_len - 2] =  self.waveforms[i][new_wave_len - 3] - 5./2.0*self.waveforms[i][new_wave_len - 2] + 2.*self.waveforms[i][new_wave_len - 1] - 1.0/2.0*self.waveforms[i][0];
        //     self.c3[i][new_wave_len - 1] =  1./2.0*(self.waveforms[i][0+1]-self.waveforms[i][new_wave_len - 2]) + 3.0/2.0*(self.waveforms[i][new_wave_len - 1]-self.waveforms[i][0]);
        //     self.c3[i][new_wave_len - 2] =  1./2.0*(self.waveforms[i][0]-self.waveforms[i][new_wave_len - 3]) + 3.0/2.0*(self.waveforms[i][new_wave_len - 2]-self.waveforms[i][new_wave_len - 1]);
        // }

    }
    //slices the read .wav into individual waveforms.
    //source_y could be avoided, by letting it take a reference to the read .wav instead
    pub(crate) fn slice(&mut self) {
        self.len = self.source_y.len();
        self.wave_len = 2048;
        self.wave_number = self.len / self.wave_len;
        self.waveforms.resize(self.wave_number, vec![0.; 2048]);
        for i in 0..self.wave_number {
            for j in 0..self.wave_len {
                self.waveforms[i][j] = self.source_y[j + self.wave_len * i];
            }
        }
    }
    pub(crate) fn optimal_coeffs(&mut self) {
        self.len = self.source_y.len();
        let new_wave_len = self.wave_len * self.amt_oversample;
        let mut even1;
        let mut even2: f32;
        let mut odd1: f32;
        let mut odd2: f32;
        self.c0.resize(
            self.mip_levels,
            vec![vec![0.; new_wave_len]; self.wave_number],
        );
        self.c1.resize(
            self.mip_levels,
            vec![vec![0.; new_wave_len]; self.wave_number],
        );
        self.c2.resize(
            self.mip_levels,
            vec![vec![0.; new_wave_len]; self.wave_number],
        );
        self.c3.resize(
            self.mip_levels,
            vec![vec![0.; new_wave_len]; self.wave_number],
        );

        for n in 0..self.mip_levels {
            //n represent mip-map levels
            for i in 0..self.wave_number {
                for j in 1..self.mips[n][0].len() - 2 {
                    even1 = self.mips[n][i][j + 1] + self.mips[n][i][j + 0];
                    odd1 = self.mips[n][i][j + 1] - self.mips[n][i][j + 0];
                    even2 = self.mips[n][i][j + 2] + self.mips[n][i][j - 1];
                    odd2 = self.mips[n][i][j + 2] - self.mips[n][i][j - 1];
                    self.c0[n][i][j] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                    self.c1[n][i][j] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                    self.c2[n][i][j] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                    self.c3[n][i][j] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
                }
            }
        }
        //makes sure the start of waveforms are handled properly

        for n in 0..self.mip_levels {
            let j = self.mips[n][0].len();
            for i in 0..self.wave_number {
                even1 = self.mips[n][i][0 + 1] + self.mips[n][i][0 + 0];
                odd1 = self.mips[n][i][0 + 1] - self.mips[n][i][0 + 0];
                even2 = self.mips[n][i][0 + 2] + self.mips[n][i][j - 1];
                odd2 = self.mips[n][i][0 + 2] - self.mips[n][i][j - 1];
                self.c0[n][i][0] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                self.c1[n][i][0] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                self.c2[n][i][0] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                self.c3[n][i][0] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
            }
        }
        //makes sure the end of waveforms are handled properly
        for n in 0..self.mip_levels {
            let j = self.mips[n][0].len();
            for i in 0..self.wave_number {
                even1 = self.mips[n][i][j - 1] + self.mips[n][i][j - 2];
                odd1 = self.mips[n][i][j - 1] - self.mips[n][i][j - 2];
                even2 = self.mips[n][i][0] + self.mips[n][i][j - 3];
                odd2 = self.mips[n][i][0] - self.mips[n][i][j - 3];
                self.c0[n][i][j - 2] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                self.c1[n][i][j - 2] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                self.c2[n][i][j - 2] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                self.c3[n][i][j - 2] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;

                even1 = self.mips[n][i][0] + self.mips[n][i][j - 1];
                odd1 = self.mips[n][i][0] - self.mips[n][i][j - 1];
                even2 = self.mips[n][i][1] + self.mips[n][i][j - 2];
                odd2 = self.mips[n][i][1] - self.mips[n][i][j - 2];
                self.c0[n][i][j - 1] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                self.c1[n][i][j - 1] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                self.c2[n][i][j - 1] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                self.c3[n][i][j - 1] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
            }
        }
    }

    pub(crate) fn static_convolve(&self, p_coeffs: &Vec<f32>, p_in: &Vec<f32>) -> Vec<f32> {
        //possibly more efficient convolution https://stackoverflow.com/questions/8424170/1d-linear-convolution-in-ansi-c-code
        //convolution could be significantly sped up by doing it in the frequency domain. from O(n^2) to O(n*log(n))
        let mut convolved: Vec<f32>;
        let new_len = p_in.len() + (p_coeffs.len() - 1) / 2;
        convolved = vec![0.; p_in.len() + p_coeffs.len()];
        //convolved.resize(p_in.len() + p_coeffs.len(), 0.);
        let mut temp = p_in.to_vec();
        temp.resize(new_len, 0.);
        //n should be the length of p_in + length of p_coeffs
        for k in 0..(new_len)
        //  position in output
        {
            for i in 0..p_coeffs.len()
            //  position in coefficients array
            {
                if k >= i {
                    convolved[k] += p_coeffs[i] * temp[k - i];
                }
            }
        }
        //trimming the result
        //remove initial group delay by taking number of coefficients - 1 / 2. Only works for odd number of coefficients
        for _i in 0..(p_coeffs.len() - 1) / 2 {
            convolved.remove(0); //maybe use drain on an iterator instead?
        }
        //trims unnecessary samples at the end
        convolved.truncate(p_in.len());
        return convolved;
    }
}
impl Default for WaveTable {
    fn default() -> WaveTable {
        WaveTable {
            source_y: Vec::with_capacity(2048 * 256),
            waveforms: Vec::with_capacity(256),
            mips: Vec::with_capacity(10),
            mip_levels: 8,
            len: 0,
            wave_number: 0,
            amt_oversample: 1,
            wave_len: 2048,
            //coeffs : Vec<Vec<f32>>, //hopefully this can make 2 vectors of f32
            //default capacity should take oversampling into account
            //capacity probably needs only to be the number of mips
            c0: Vec::with_capacity(10),
            c1: Vec::with_capacity(10),
            c2: Vec::with_capacity(10),
            c3: Vec::with_capacity(10),
            upsample_fir: vec![
                5.807e-05,
                0.00015957,
                0.00017629,
                4.1774e-06,
                -0.00021049,
                -0.0001965,
                2.3485e-05,
                9.5114e-05,
                -0.00011663,
                -0.00024653,
                -1.8106e-05,
                0.00021017,
                3.0079e-06,
                -0.00032069,
                -0.00014625,
                0.00029734,
                0.00020506,
                -0.00034762,
                -0.00036956,
                0.00029091,
                0.0004813,
                -0.00025542,
                -0.00065389,
                0.0001256,
                0.00077988,
                1.7544e-05,
                -0.0009229,
                -0.00024499,
                0.0010065,
                0.00050125,
                -0.0010623,
                -0.00082416,
                0.0010349,
                0.0011698,
                -0.00093752,
                -0.0015501,
                0.00073146,
                0.001924,
                -0.00042358,
                -0.0022835,
                -8.6855e-06,
                0.0025865,
                0.00055521,
                -0.0028116,
                -0.0012206,
                0.0029163,
                0.0019844,
                -0.0028726,
                -0.0028315,
                0.0026421,
                0.003727,
                -0.0021982,
                -0.0046357,
                0.0015123,
                0.0055074,
                -0.00056789,
                -0.0062889,
                -0.00064776,
                0.0069164,
                0.0021342,
                -0.0073232,
                -0.0038835,
                0.0074351,
                0.005874,
                -0.0071756,
                -0.0080748,
                0.0064621,
                0.010443,
                -0.0052066,
                -0.012928,
                0.0033099,
                0.015469,
                -0.00065173,
                -0.018001,
                -0.0029281,
                0.020454,
                0.0076684,
                -0.022758,
                -0.013972,
                0.024844,
                0.0226,
                -0.026649,
                -0.035204,
                0.028117,
                0.056141,
                -0.029201,
                -0.10152,
                0.029865,
                0.31677,
                0.46991,
                0.31677,
                0.029865,
                -0.10152,
                -0.029201,
                0.056141,
                0.028117,
                -0.035204,
                -0.026649,
                0.0226,
                0.024844,
                -0.013972,
                -0.022758,
                0.0076684,
                0.020454,
                -0.0029281,
                -0.018001,
                -0.00065173,
                0.015469,
                0.0033099,
                -0.012928,
                -0.0052066,
                0.010443,
                0.0064621,
                -0.0080748,
                -0.0071756,
                0.005874,
                0.0074351,
                -0.0038835,
                -0.0073232,
                0.0021342,
                0.0069164,
                -0.00064776,
                -0.0062889,
                -0.00056789,
                0.0055074,
                0.0015123,
                -0.0046357,
                -0.0021982,
                0.003727,
                0.0026421,
                -0.0028315,
                -0.0028726,
                0.0019844,
                0.0029163,
                -0.0012206,
                -0.0028116,
                0.00055521,
                0.0025865,
                -8.6855e-06,
                -0.0022835,
                -0.00042358,
                0.001924,
                0.00073146,
                -0.0015501,
                -0.00093752,
                0.0011698,
                0.0010349,
                -0.00082416,
                -0.0010623,
                0.00050125,
                0.0010065,
                -0.00024499,
                -0.0009229,
                1.7544e-05,
                0.00077988,
                0.0001256,
                -0.00065389,
                -0.00025542,
                0.0004813,
                0.00029091,
                -0.00036956,
                -0.00034762,
                0.00020506,
                0.00029734,
                -0.00014625,
                -0.00032069,
                3.0079e-06,
                0.00021017,
                -1.8106e-05,
                -0.00024653,
                -0.00011663,
                9.5114e-05,
                2.3485e-05,
                -0.0001965,
                -0.00021049,
                4.1774e-06,
                0.00017629,
                0.00015957,
                5.807e-05,
            ],
            downsample_fir: vec![
                6.6501e-05,
                0.00016925,
                0.00013554,
                -9.6855e-05,
                -0.00025644,
                -0.00011532,
                6.7101e-05,
                -4.2646e-05,
                -0.00020891,
                -5.5997e-05,
                0.00014325,
                -3.4041e-05,
                -0.0002427,
                -5.113e-06,
                0.00023407,
                -5.8992e-05,
                -0.00031185,
                6.2543e-05,
                0.0003345,
                -0.00012665,
                -0.00040184,
                0.00016754,
                0.00043956,
                -0.00024865,
                -0.00049771,
                0.0003276,
                0.00053631,
                -0.00043767,
                -0.00057921,
                0.0005572,
                0.0006033,
                -0.00070362,
                -0.00061865,
                0.00086496,
                0.00061014,
                -0.00105,
                -0.00058064,
                0.0012506,
                0.00051885,
                -0.0014698,
                -0.00042359,
                0.0017007,
                0.00028602,
                -0.0019419,
                -0.00010262,
                0.0021861,
                -0.00013333,
                -0.0024277,
                0.0004261,
                0.0026582,
                -0.00078058,
                -0.0028689,
                0.0012001,
                0.0030488,
                -0.0016878,
                -0.0031867,
                0.0022447,
                0.0032692,
                -0.0028717,
                -0.0032825,
                0.0035672,
                0.0032112,
                -0.0043288,
                -0.003039,
                0.0051523,
                0.0027483,
                -0.0060318,
                -0.0023202,
                0.0069598,
                0.0017343,
                -0.0079273,
                -0.00096783,
                0.0089238,
                -5.1968e-06,
                -0.0099377,
                0.0012151,
                0.010956,
                -0.0026994,
                -0.011965,
                0.0045069,
                0.012951,
                -0.0067036,
                -0.013899,
                0.0093857,
                0.014795,
                -0.012698,
                -0.015624,
                0.016878,
                0.016373,
                -0.022335,
                -0.01703,
                0.029851,
                0.017583,
                -0.041109,
                -0.018024,
                0.060505,
                0.018345,
                -0.10419,
                -0.01854,
                0.31767,
                0.5186,
                0.31767,
                -0.01854,
                -0.10419,
                0.018345,
                0.060505,
                -0.018024,
                -0.041109,
                0.017583,
                0.029851,
                -0.01703,
                -0.022335,
                0.016373,
                0.016878,
                -0.015624,
                -0.012698,
                0.014795,
                0.0093857,
                -0.013899,
                -0.0067036,
                0.012951,
                0.0045069,
                -0.011965,
                -0.0026994,
                0.010956,
                0.0012151,
                -0.0099377,
                -5.1968e-06,
                0.0089238,
                -0.00096783,
                -0.0079273,
                0.0017343,
                0.0069598,
                -0.0023202,
                -0.0060318,
                0.0027483,
                0.0051523,
                -0.003039,
                -0.0043288,
                0.0032112,
                0.0035672,
                -0.0032825,
                -0.0028717,
                0.0032692,
                0.0022447,
                -0.0031867,
                -0.0016878,
                0.0030488,
                0.0012001,
                -0.0028689,
                -0.00078058,
                0.0026582,
                0.0004261,
                -0.0024277,
                -0.00013333,
                0.0021861,
                -0.00010262,
                -0.0019419,
                0.00028602,
                0.0017007,
                -0.00042359,
                -0.0014698,
                0.00051885,
                0.0012506,
                -0.00058064,
                -0.00105,
                0.00061014,
                0.00086496,
                -0.00061865,
                -0.00070362,
                0.0006033,
                0.0005572,
                -0.00057921,
                -0.00043767,
                0.00053631,
                0.0003276,
                -0.00049771,
                -0.00024865,
                0.00043956,
                0.00016754,
                -0.00040184,
                -0.00012665,
                0.0003345,
                6.2543e-05,
                -0.00031185,
                -5.8992e-05,
                0.00023407,
                -5.113e-06,
                -0.0002427,
                -3.4041e-05,
                0.00014325,
                -5.5997e-05,
                -0.00020891,
                -4.2646e-05,
                6.7101e-05,
                -0.00011532,
                -0.00025644,
                -9.6855e-05,
                0.00013554,
                0.00016925,
                6.6501e-05,
            ],
        }
    }
}