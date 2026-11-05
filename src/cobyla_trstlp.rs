//! Direct translation of `trstlp` from Dragology's pinned `src/math/cobyla.js`.
//! Source revision: b145bdd56530f7cfa9a61bf6a7170459d2f140e1.
//! The source is JSXGraph/jcobyla, used under its MIT license. Arrays intentionally
//! remain one-based, and dot products retain the source's summation order.

/*
Copyright 2008-2025
Matthias Ehmann,
Carsten Miller,
Reinhard Oldenburg,
Andreas Walter,
Alfred Wassermann

This file is part of JSXGraph.

JSXGraph is free software dual licensed under the GNU LGPL or MIT License.

You can redistribute it and/or modify it under the terms of the

* GNU Lesser General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version
OR
* MIT License: https://github.com/jsxgraph/jsxgraph/blob/master/LICENSE.MIT

JSXGraph is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Lesser General Public License for more details.

You should have received a copy of the GNU Lesser General Public License and
the MIT License along with JSXGraph. If not, see <https://www.gnu.org/licenses/>
and <https://opensource.org/licenses/MIT/>.

This is a port of jcobyla

- to JavaScript by Reihard Oldenburg and
- to JSXGraph by Alfred Wassermann
- optimized by Andreas Walter
*/
/*
 * jcobyla
 *
 * The MIT License
 *
 * Copyright (c) 2012 Anders Gustafsson, Cureos AB.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files
 * (the 'Software'), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge,
 * publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
 * MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
 * WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 *
 * Remarks:
 *
 * The original Fortran 77 version of this code was by Michael Powell (M.J.D.Powell @ damtp.cam.ac.uk)
 * The Fortran 90 version was by Alan Miller (Alan.Miller @ vic.cmis.csiro.au). Latest revision - 30 October 1998
 */

// The first stage minimizes the greatest constraint violation in the trust
// region. The second stage minimizes the linear objective without increasing
// that violation. `true` is the original IFULL=1 result.
#[allow(unused_assignments)]
// Keep one-based indexed loops in source order for direct numerical comparison.
#[allow(clippy::needless_range_loop)]
pub(super) fn trstlp(
    n: usize,
    m: usize,
    a: &[Vec<f64>],
    b: &[f64],
    rho: f64,
    dx: &mut [f64],
) -> bool {
    let mut temp: f64 = 0.;
    let mut nactx = 0;
    let mut resold = 0.;
    let mut z = vec![vec![0.; 1 + n]; 1 + n];
    let mut zdota = vec![0.; 2 + m];
    let mut vmultc = vec![0.; 2 + m];
    let mut sdirn = vec![0.; 1 + n];
    let mut dxnew = vec![0.; 1 + n];
    let mut vmultd = vec![0.; 2 + m];
    let mut iact = vec![0usize; 2 + m];
    let mut mcon = m;
    let mut nact = 0;
    let mut icon = 0;
    let mut resmax = 0.;
    let mut k: usize;
    let mut first = true;
    let mut optold: f64;
    let mut icount: usize;
    let mut step: f64;
    let mut stpful: f64;
    let mut optnew: f64;
    let mut ratio: f64;
    let mut isave: usize;
    let mut vsave: f64;
    let mut total: f64;
    let mut kp: usize;
    let mut kk: usize;
    let mut sp: f64;
    let mut alpha: f64;
    let mut beta: f64;
    let mut tot: f64;
    let mut spabs: f64;
    let mut acca: f64;
    let mut accb: f64;
    let mut zdotv: f64;
    let mut zdvabs: f64;
    let mut kw: usize;
    let mut dd: f64;
    let mut ss: f64;
    let mut sd: f64;
    let mut zdotw: f64;
    let mut zdwabs: f64;
    let mut sumabs: f64;
    let mut tempa: f64;

    for i in 1..=n {
        z[i][i] = 1.;
        dx[i] = 0.;
    }
    if m >= 1 {
        for k in 1..=m {
            if b[k] > resmax {
                resmax = b[k];
                icon = k;
            }
        }
        for k in 1..=m {
            iact[k] = k;
            vmultc[k] = resmax - b[k];
        }
    }

    // Three consecutive iterations without reducing the objective or enlarging
    // the active set end a stage, preventing cycling as in the source.
    loop {
        'l60: loop {
            if !first || (first && resmax == 0.) {
                mcon = m + 1;
                icon = mcon;
                iact[mcon] = mcon;
                vmultc[mcon] = 0.;
            }
            first = false;
            optold = 0.;
            icount = 0;
            step = 0.;
            stpful = 0.;

            'l70: loop {
                optnew = if mcon == m {
                    resmax
                } else {
                    -dot_vector_column(dx, a, mcon, n)
                };
                if icount == 0 || optnew < optold {
                    optold = optnew;
                    nactx = nact;
                    icount = 3;
                } else if nact > nactx {
                    nactx = nact;
                    icount = 3;
                } else {
                    icount -= 1;
                }
                if icount == 0 {
                    break 'l60;
                }

                // Add IACT(ICON) to the active set when ICON > NACT. Givens
                // rotations make the remaining columns orthogonal to its
                // gradient, with the source's roundoff checks unchanged.
                ratio = 0.;
                if icon <= nact {
                    if icon < nact {
                        // Delete the constraint IACT(ICON) from the active set.
                        isave = iact[icon];
                        vsave = vmultc[icon];
                        k = icon;
                        loop {
                            kp = k + 1;
                            kk = iact[kp];
                            sp = dot_columns(&z, k, a, kk, n);
                            temp = js_hypot(sp, zdota[kp]);
                            alpha = zdota[kp] / temp;
                            beta = sp / temp;
                            zdota[kp] = alpha * zdota[k];
                            zdota[k] = temp;
                            for i in 1..=n {
                                temp = alpha * z[i][kp] + beta * z[i][k];
                                z[i][kp] = alpha * z[i][k] - beta * z[i][kp];
                                z[i][k] = temp;
                            }
                            iact[k] = kk;
                            vmultc[k] = vmultc[kp];
                            k = kp;
                            if k >= nact {
                                break;
                            }
                        }
                        iact[k] = isave;
                        vmultc[k] = vsave;
                    }
                    nact -= 1;

                    if mcon > m {
                        // Pick the next search direction of stage two.
                        temp = 1. / zdota[nact];
                        for k in 1..=n {
                            sdirn[k] = temp * z[k][nact];
                        }
                    } else {
                        temp = dot_vector_column(&sdirn, &z, nact + 1, n);
                        for k in 1..=n {
                            sdirn[k] -= temp * z[k][nact + 1];
                        }
                    }
                } else {
                    kk = iact[icon];
                    for k in 1..=n {
                        dxnew[k] = a[k][kk];
                    }
                    tot = 0.;
                    k = n;
                    while k > nact {
                        sp = 0.;
                        spabs = 0.;
                        for i in 1..=n {
                            temp = z[i][k] * dxnew[i];
                            sp += temp;
                            spabs += temp.abs();
                        }
                        acca = spabs + 0.1 * sp.abs();
                        accb = spabs + 0.2 * sp.abs();
                        if spabs >= acca || acca >= accb {
                            sp = 0.;
                        }
                        if tot == 0. {
                            tot = sp;
                        } else {
                            kp = k + 1;
                            temp = js_hypot(sp, tot);
                            alpha = sp / temp;
                            beta = tot / temp;
                            tot = temp;
                            for i in 1..=n {
                                temp = alpha * z[i][k] + beta * z[i][kp];
                                z[i][kp] = alpha * z[i][kp] - beta * z[i][k];
                                z[i][k] = temp;
                            }
                        }
                        k -= 1;
                    }

                    if tot == 0. {
                        // A new gradient is a linear combination of old active
                        // gradients. Compute its multipliers and choose a deletion.
                        ratio = -1.;
                        k = nact;
                        loop {
                            zdotv = 0.;
                            zdvabs = 0.;
                            for i in 1..=n {
                                temp = z[i][k] * dxnew[i];
                                zdotv += temp;
                                zdvabs += temp.abs();
                            }
                            acca = zdvabs + 0.1 * zdotv.abs();
                            accb = zdvabs + 0.2 * zdotv.abs();
                            if zdvabs < acca && acca < accb {
                                temp = zdotv / zdota[k];
                                if temp > 0. && iact[k] <= m {
                                    tempa = vmultc[k] / temp;
                                    if ratio < 0. || tempa < ratio {
                                        ratio = tempa;
                                    }
                                }
                                if k >= 2 {
                                    kw = iact[k];
                                    for i in 1..=n {
                                        dxnew[i] -= temp * a[i][kw];
                                    }
                                }
                                vmultd[k] = temp;
                            } else {
                                vmultd[k] = 0.;
                            }
                            // JavaScript's `while (--k > 0)` also exits from k=0.
                            if k <= 1 {
                                break;
                            }
                            k -= 1;
                        }
                        if ratio < 0. {
                            break 'l60;
                        }

                        // Revise multipliers, move the replacement to the end,
                        // and calculate the new ZDOTA(NACT).
                        for k in 1..=nact {
                            vmultc[k] = js_max(0., vmultc[k] - ratio * vmultd[k]);
                        }
                        if icon < nact {
                            isave = iact[icon];
                            vsave = vmultc[icon];
                            k = icon;
                            loop {
                                kp = k + 1;
                                kw = iact[kp];
                                sp = dot_columns(&z, k, a, kw, n);
                                temp = js_hypot(sp, zdota[kp]);
                                alpha = zdota[kp] / temp;
                                beta = sp / temp;
                                zdota[kp] = alpha * zdota[k];
                                zdota[k] = temp;
                                for i in 1..=n {
                                    temp = alpha * z[i][kp] + beta * z[i][k];
                                    z[i][kp] = alpha * z[i][k] - beta * z[i][kp];
                                    z[i][k] = temp;
                                }
                                iact[k] = kw;
                                vmultc[k] = vmultc[kp];
                                k = kp;
                                if k >= nact {
                                    break;
                                }
                            }
                            iact[k] = isave;
                            vmultc[k] = vsave;
                        }
                        temp = dot_columns(&z, nact, a, kk, n);
                        if temp == 0. {
                            break 'l60;
                        }
                        zdota[nact] = temp;
                        vmultc[icon] = 0.;
                        vmultc[nact] = ratio;
                    } else {
                        // Add the constraint without deleting another.
                        nact += 1;
                        zdota[nact] = tot;
                        vmultc[icon] = vmultc[nact];
                        vmultc[nact] = 0.;
                    }

                    // Keep the objective as the last active constraint in stage2.
                    iact[icon] = iact[nact];
                    iact[nact] = kk;
                    if mcon > m && kk != mcon {
                        k = nact - 1;
                        sp = dot_columns(&z, k, a, kk, n);
                        temp = js_hypot(sp, zdota[nact]);
                        alpha = zdota[nact] / temp;
                        beta = sp / temp;
                        zdota[nact] = alpha * zdota[k];
                        zdota[k] = temp;
                        for i in 1..=n {
                            temp = alpha * z[i][nact] + beta * z[i][k];
                            z[i][nact] = alpha * z[i][k] - beta * z[i][nact];
                            z[i][k] = temp;
                        }
                        iact[nact] = iact[k];
                        iact[k] = kk;
                        temp = vmultc[k];
                        vmultc[k] = vmultc[nact];
                        vmultc[nact] = temp;
                    }

                    if mcon > m {
                        temp = 1. / zdota[nact];
                        for k in 1..=n {
                            sdirn[k] = temp * z[k][nact];
                        }
                    } else {
                        kk = iact[nact];
                        temp = (dot_vector_column(&sdirn, a, kk, n) - 1.) / zdota[nact];
                        for k in 1..=n {
                            sdirn[k] -= temp * z[k][nact];
                        }
                    }
                }

                // Step to the trust-region boundary, or to zero violation.
                // Retain both source 1e-6 tests against harmless underflow.
                dd = rho * rho;
                sd = 0.;
                ss = 0.;
                for i in 1..=n {
                    if dx[i].abs() >= 1.0e-6 * rho {
                        dd -= dx[i] * dx[i];
                    }
                    sd += dx[i] * sdirn[i];
                    ss += sdirn[i] * sdirn[i];
                }
                if dd <= 0. {
                    break 'l60;
                }
                temp = (ss * dd).sqrt();
                if sd.abs() >= 1.0e-6 * temp {
                    temp = (ss * dd + sd * sd).sqrt();
                }
                stpful = dd / (temp + sd);
                step = stpful;
                if mcon == m {
                    acca = step + 0.1 * resmax;
                    accb = step + 0.2 * resmax;
                    if step >= acca || acca >= accb {
                        break 'l70;
                    }
                    step = js_min(step, resmax);
                }

                for k in 1..=n {
                    dxnew[k] = dx[k] + step * sdirn[k];
                }
                if mcon == m {
                    resold = resmax;
                    resmax = 0.;
                    for k in 1..=nact {
                        kk = iact[k];
                        temp = b[kk] - dot_vector_column(&dxnew, a, kk, n);
                        resmax = js_max(resmax, temp);
                    }
                }

                // Multipliers for DXNEW, with the original roundoff tests.
                k = nact;
                loop {
                    zdotw = 0.;
                    zdwabs = 0.;
                    for i in 1..=n {
                        temp = z[i][k] * dxnew[i];
                        zdotw += temp;
                        zdwabs += temp.abs();
                    }
                    acca = zdwabs + 0.1 * zdotw.abs();
                    accb = zdwabs + 0.2 * zdotw.abs();
                    if zdwabs >= acca || acca >= accb {
                        zdotw = 0.;
                    }
                    vmultd[k] = zdotw / zdota[k];
                    if k >= 2 {
                        kk = iact[k];
                        for i in 1..=n {
                            dxnew[i] -= vmultd[k] * a[i][kk];
                        }
                    }
                    // Source do/while is `while (k-- >= 2)`.
                    if k < 2 {
                        break;
                    }
                    k -= 1;
                }
                if mcon > m {
                    vmultd[nact] = js_max(0., vmultd[nact]);
                }

                // Complete VMULTD with the new inactive constraint residuals.
                for k in 1..=n {
                    dxnew[k] = dx[k] + step * sdirn[k];
                }
                if mcon > nact {
                    for k in nact + 1..=mcon {
                        kk = iact[k];
                        total = resmax - b[kk];
                        sumabs = resmax + b[kk].abs();
                        for i in 1..=n {
                            temp = a[i][kk] * dxnew[i];
                            total += temp;
                            sumabs += temp.abs();
                        }
                        acca = sumabs + 0.1 * total.abs();
                        accb = sumabs + 0.2 * total.abs();
                        if sumabs >= acca || acca >= accb {
                            total = 0.;
                        }
                        vmultd[k] = total;
                    }
                }

                ratio = 1.;
                icon = 0;
                for k in 1..=mcon {
                    if vmultd[k] < 0. {
                        temp = vmultc[k] / (vmultc[k] - vmultd[k]);
                        if temp < ratio {
                            ratio = temp;
                            icon = k;
                        }
                    }
                }

                temp = 1. - ratio;
                for k in 1..=n {
                    dx[k] = temp * dx[k] + ratio * dxnew[k];
                }
                for k in 1..=mcon {
                    vmultc[k] = js_max(0., temp * vmultc[k] + ratio * vmultd[k]);
                }
                if mcon == m {
                    resmax = resold + ratio * (resmax - resold);
                }
                if icon == 0 {
                    break 'l70;
                }
            }
            if step == stpful {
                return true;
            }
        }
        // Try stage two before returning a step shorter than RHO.
        if mcon != m {
            break;
        }
    }
    false
}

fn dot_vector_column(lhs: &[f64], rhs: &[Vec<f64>], column: usize, n: usize) -> f64 {
    let mut sum = 0.;
    for i in 1..=n {
        sum += lhs[i] * rhs[i][column];
    }
    sum
}
fn dot_columns(lhs: &[Vec<f64>], left: usize, rhs: &[Vec<f64>], right: usize, n: usize) -> f64 {
    let mut sum = 0.;
    for i in 1..=n {
        sum += lhs[i][left] * rhs[i][right];
    }
    sum
}
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0. && b == 0. {
        if a.is_sign_positive() || b.is_sign_positive() {
            0.
        } else {
            -0.
        }
    } else if a > b {
        a
    } else {
        b
    }
}
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0. && b == 0. {
        if a.is_sign_negative() || b.is_sign_negative() {
            -0.
        } else {
            0.
        }
    } else if a < b {
        a
    } else {
        b
    }
}

/*
The Math.hypot helper below is translated from:
https://github.com/nodejs/node/blob/v22.14.0/deps/v8/src/builtins/math.tq
Copyright 2019 the V8 project authors. All rights reserved. Use of this
source code is governed by a BSD-style license:

Copyright 2014, the V8 project authors. All rights reserved.
Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above
      copyright notice, this list of conditions and the following
      disclaimer in the documentation and/or other materials provided
      with the distribution.
    * Neither the name of Google Inc. nor the names of its
      contributors may be used to endorse or promote products derived
      from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/
// Match Math.hypot in the reference Node/V8 engine: normalize before summing
// squares. libc hypot can round a few ulps differently, affecting rank tests.
fn js_hypot(a: f64, b: f64) -> f64 {
    let a = a.abs();
    let b = b.abs();
    if a.is_infinite() || b.is_infinite() {
        return f64::INFINITY;
    }
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    let max = if a > b { a } else { b };
    if max == 0. {
        return 0.;
    }
    let mut sum = 0.;
    let mut compensation = 0.;
    for value in [a, b] {
        let n = value / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        name: &'static str,
        n: usize,
        m: usize,
        a: Vec<Vec<f64>>,
        b: Vec<f64>,
        rho: f64,
        full: bool,
        dx: Vec<f64>,
    }
    #[test]
    fn trust_region_steps_match_original_javascript_for_active_and_degenerate_sets() {
        // Executed original Nlp.trstlp at pinned revision b145bdd56530f7cfa9a61bf6a7170459d2f140e1
        // under Node22.14.0. Regenerate with:
        // node tools/upstream-parity/reference-trust-region.cjs /path/to/upstream
        // The generated_154 rank-deficient case specifically
        // guards Math.hypot rounding: replacing js_hypot with libc hypot changes
        // its step by0.35 even though both satisfy the trust-region bound.
        let fixtures = vec![
            Fixture {
                name: "unconstrained",
                n: 2,
                m: 0,
                a: vec![vec![0.0, 0.0], vec![0.0, 3.0], vec![0.0, 4.0]],
                b: vec![0.0, 0.0],
                rho: 1.0,
                full: true,
                dx: vec![0.0, 0.6, 0.8000000000000002],
            },
            Fixture {
                name: "zero_objective",
                n: 2,
                m: 0,
                a: vec![vec![0.0, 0.0], vec![0.0, 0.0], vec![0.0, 0.0]],
                b: vec![0.0, 0.0],
                rho: 1.0,
                full: false,
                dx: vec![0.0, 0.0, 0.0],
            },
            Fixture {
                name: "active_half_plane",
                n: 2,
                m: 1,
                a: vec![
                    vec![0.0, 0.0, 0.0],
                    vec![0.0, 1.0, -1.0],
                    vec![0.0, 0.0, 2.0],
                ],
                b: vec![0.0, 0.3, 0.0],
                rho: 1.0,
                full: true,
                dx: vec![0.0, 0.3, 0.9539392014169457],
            },
            Fixture {
                name: "infeasible_opposites",
                n: 2,
                m: 2,
                a: vec![
                    vec![0.0, 0.0, 0.0, 0.0],
                    vec![0.0, 1.0, -1.0, 0.0],
                    vec![0.0, 0.0, 0.0, 1.0],
                ],
                b: vec![0.0, 1.0, 1.0, 0.0],
                rho: 0.5,
                full: true,
                dx: vec![0.0, 0.0, 0.5],
            },
            Fixture {
                name: "finite_constrained_optimum",
                n: 1,
                m: 1,
                a: vec![vec![0.0, 0.0, 0.0], vec![0.0, -1.0, 1.0]],
                b: vec![0.0, -0.2, 0.0],
                rho: 1.0,
                full: false,
                dx: vec![0.0, 0.2],
            },
            Fixture {
                name: "box_optimum_inside_radius",
                n: 2,
                m: 2,
                a: vec![
                    vec![0.0, 0.0, 0.0, 0.0],
                    vec![0.0, -1.0, 0.0, 1.0],
                    vec![0.0, 0.0, -1.0, 1.0],
                ],
                b: vec![0.0, -0.2, -0.3, 0.0],
                rho: 1.0,
                full: false,
                dx: vec![0.0, 0.20000000000000004, 0.3],
            },
            Fixture {
                name: "generated_7",
                n: 3,
                m: 8,
                a: vec![
                    vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    vec![0.0, 2.0, 4.0, 0.25, 0.0, 0.25, -0.25, -0.5, 1.5, -0.75],
                    vec![0.0, 0.75, 1.5, -1.5, 1.75, 1.75, -1.25, 1.0, -0.25, 0.25],
                    vec![0.0, -2.0, -4.0, -1.5, -1.75, 0.5, 0.0, 0.75, 0.75, -1.25],
                ],
                b: vec![0.0, 0.5, 1.0, 0.5, -1.0, -0.25, 0.0, 0.5, 0.75, 0.0],
                rho: 1.2316769794460973,
                full: false,
                dx: vec![
                    0.0,
                    0.34558823529411775,
                    0.35294117647058837,
                    -0.2769607843137256,
                ],
            },
            Fixture {
                name: "generated_21",
                n: 2,
                m: 7,
                a: vec![
                    vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    vec![0.0, -0.75, -1.5, 1.75, 1.75, 1.0, 0.75, -1.25, -1.25],
                    vec![0.0, 0.0, 0.0, 2.0, -0.25, -1.75, -0.5, 0.0, -0.75],
                ],
                b: vec![0.0, -0.5, -1.0, 1.25, 0.75, -1.5, 0.75, 0.0, 0.0],
                rho: 6.232655557829251,
                full: false,
                dx: vec![0.0, 0.38636363636363635, 0.04545454545454541],
            },
            Fixture {
                name: "generated_117",
                n: 3,
                m: 4,
                a: vec![
                    vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    vec![0.0, -2.0, -1.0, 2.0, 0.0, 1.25],
                    vec![0.0, -0.5, -0.25, -1.5, 1.25, 1.75],
                    vec![0.0, 1.75, -1.75, 1.5, -0.75, -1.0],
                ],
                b: vec![0.0, -0.75, -1.25, -0.5, -0.25, 0.0],
                rho: 0.028212970335109305,
                full: true,
                dx: vec![
                    0.0,
                    0.014869540969618228,
                    0.020817357357465517,
                    -0.011895632775694583,
                ],
            },
            Fixture {
                name: "generated_154",
                n: 5,
                m: 4,
                a: vec![
                    vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    vec![0.0, -1.25, 1.25, -0.25, 2.0, 1.5],
                    vec![0.0, 0.75, -0.75, -1.0, 1.75, 1.75],
                    vec![0.0, 0.75, -0.75, 1.5, -1.5, 1.75],
                    vec![0.0, 0.5, -0.5, -0.5, 0.75, -0.75],
                    vec![0.0, 0.75, -0.75, 0.0, -0.25, -1.0],
                ],
                b: vec![0.0, 0.25, 0.25, 1.25, 0.5, 0.0],
                rho: 1.3601111809844966,
                full: true,
                dx: vec![
                    0.0,
                    0.7928668963728549,
                    0.3722528188067892,
                    1.0377872367321246,
                    -0.02757737560363249,
                    -0.07021031118173396,
                ],
            },
            Fixture {
                name: "generated_197",
                n: 3,
                m: 1,
                a: vec![
                    vec![0.0, 0.0, 0.0],
                    vec![0.0, 1.25, -0.5],
                    vec![0.0, 1.75, -0.5],
                    vec![0.0, -1.0, 0.75],
                ],
                b: vec![0.0, 0.0, 0.0],
                rho: 0.10033084395017478,
                full: true,
                dx: vec![
                    0.0,
                    1.6708441955033644e-17,
                    0.04977803840188487,
                    0.08711156720329855,
                ],
            },
        ];
        for fixture in fixtures {
            let mut dx = vec![0.; fixture.n + 1];
            let full = trstlp(
                fixture.n,
                fixture.m,
                &fixture.a,
                &fixture.b,
                fixture.rho,
                &mut dx,
            );
            assert_eq!(full, fixture.full, "{}", fixture.name);
            for (i, (actual, expected)) in dx.iter().zip(&fixture.dx).enumerate() {
                assert!(
                    (actual - expected).abs() <= 2e-14 * (1. + fixture.rho),
                    "{} coordinate{i}: expected{expected}, got{actual}",
                    fixture.name
                );
            }
            let length_squared: f64 = dx[1..].iter().map(|x| x * x).sum();
            assert!(
                length_squared <= fixture.rho * fixture.rho * (1. + 1e-12),
                "{}",
                fixture.name
            );
        }
    }
}
