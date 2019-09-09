// move the firs in interp.rs in here, so they're not needlessly copied twice

    pub const UPSAMPLE_FIR : [f32;179]= [
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
    ];
    pub const DOWNSAMPLE_FIR : [f32;201] = [
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
    ];