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

//! A direct translation of the COBYLA implementation used by Dragology.
//! Source: declarative-dragging/dragology b145bdd56530f7cfa9a61bf6a7170459d2f140e1,
//! src/math/cobyla.js. One-based work arrays and evaluation order are intentional.

#[path = "cobyla_trstlp.rs"]
mod trust_region;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Normal,
    MaxEvaluations,
    RoundingErrors,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub status: Status,
    pub evaluations: usize,
}

/// Minimize `calc` with each constraint nonnegative; update `x` in place.
/// A zero evaluation budget still performs the initial evaluation, as upstream does.
pub(crate) fn find_minimum(
    mut calc: impl FnMut(&[f64], &mut [f64]) -> f64,
    constraint_count: usize,
    x: &mut [f64],
    rhobeg: f64,
    rhoend: f64,
    maxfun: usize,
) -> Outcome {
    let n = x.len();
    let m = constraint_count;
    let mut internal_x = vec![0.0; n + 1];
    internal_x[1..].copy_from_slice(x);
    let mut calc_one_based = |point: &[f64], con: &mut [f64]| {
        let mut constraints = vec![0.0; m];
        let value = calc(&point[1..], &mut constraints);
        con[1..=m].copy_from_slice(&constraints);
        value
    };
    let (status, evaluations) = cobylb(
        &mut calc_one_based,
        n,
        m,
        m + 2,
        &mut internal_x,
        rhobeg,
        rhoend,
        maxfun,
        false,
    );
    x.copy_from_slice(&internal_x[1..]);
    Outcome {
        status: match status {
            0 => Status::Normal,
            1 => Status::MaxEvaluations,
            2 => Status::RoundingErrors,
            _ => unreachable!(),
        },
        evaluations,
    }
}

// These scalar locals follow the original Fortran control flow. Some initial
// values are overwritten before use on all paths; keeping them makes comparison
// with the upstream JavaScript translation straightforward. Explicit assignment
// operand order is retained even where *= would normally be idiomatic.
#[allow(
    unused_assignments,
    unused_parens,
    non_snake_case,
    clippy::too_many_arguments,
    clippy::assign_op_pattern
)]
fn cobylb(
    calcfc: &mut impl FnMut(&[f64], &mut [f64]) -> f64,
    n: usize,
    m: usize,
    mpp: usize,
    x: &mut [f64],
    rhobeg: f64,
    rhoend: f64,
    maxfun: usize,
    testForRoundingErrors: bool,
) -> (i32, usize) {
    let mut status: i32 = (-1);
    let alpha: f64 = 0.25;
    let beta: f64 = 2.1;
    let gamma: f64 = 0.5;
    let delta: f64 = 1.1;
    let mut f: f64 = 0.0;
    let mut resmax: f64 = 0.0;
    let mut total: f64 = 0.0;
    let np: usize = (n + 1);
    let mp: usize = (m + 1);
    let mut rho: f64 = rhobeg;
    let mut parmu: f64 = 0.0;
    let mut iflag: bool = false;
    let mut ifull: bool = false;
    let mut parsig: f64 = 0.0;
    let mut prerec: f64 = 0.0;
    let mut prerem: f64 = 0.0;
    let mut con: Vec<f64> = vec![0.0; (1 + mpp)];
    let mut sim: Vec<Vec<f64>> = vec![vec![0.0; (1 + np)]; (1 + n)];
    let mut simi: Vec<Vec<f64>> = vec![vec![0.0; (1 + n)]; (1 + n)];
    let mut datmat: Vec<Vec<f64>> = vec![vec![0.0; (1 + np)]; (1 + mpp)];
    let mut a: Vec<Vec<f64>> = vec![vec![0.0; (1 + mp)]; (1 + n)];
    let mut vsig: Vec<f64> = vec![0.0; (1 + n)];
    let mut veta: Vec<f64> = vec![0.0; (1 + n)];
    let mut sigbar: Vec<f64> = vec![0.0; (1 + n)];
    let mut dx: Vec<f64> = vec![0.0; (1 + n)];
    let mut w: Vec<f64> = vec![0.0; (1 + n)];
    let mut i: usize = 0;
    let mut j: usize = 0;
    let mut k: usize = 0;
    let mut l: usize = 0;
    let mut temp: f64 = 0.0;
    let mut tempa: f64 = 0.0;
    let mut nfvals: usize = 0;
    let mut jdrop: usize = 0;
    let mut ibrnch: bool = false;
    let mut skipVertexIdent: bool = false;
    let mut phimin: f64 = 0.0;
    let mut nbest: usize = 0;
    let mut error: f64 = 0.0;
    let mut pareta: f64 = 0.0;
    let mut wsig: f64 = 0.0;
    let mut weta: f64 = 0.0;
    let mut cvmaxp: f64 = 0.0;
    let mut cvmaxm: f64 = 0.0;
    let mut dxsign: f64 = 0.0;
    let mut resnew: f64 = 0.0;
    let mut barmu: f64 = 0.0;
    let mut phi: f64 = 0.0;
    let mut vmold: f64 = 0.0;
    let mut vmnew: f64 = 0.0;
    let mut trured: f64 = 0.0;
    let mut ratio: f64 = 0.0;
    let mut edgmax: f64 = 0.0;
    let mut cmin: f64 = 0.0;
    let mut cmax: f64 = 0.0;
    let mut denom: f64 = 0.0;

    // SIM stores the best vertex in its last column and the remaining vertex
    // displacements in its leading square matrix. SIMI stores that inverse.
    nfvals = 0;
    temp = (1.0 / rho);
    i = 1;
    while (i <= n) {
        sim[i][np] = x[i];
        sim[i][i] = rho;
        simi[i][i] = temp;
        i += 1;
    }
    jdrop = np;
    ibrnch = false;
    // L40: evaluate the next trial point, including initial simplex vertices.
    'l_40: loop {
        if ((nfvals >= maxfun) && (nfvals > 0)) {
            status = 1;
            break 'l_40;
        }
        nfvals += 1;
        f = calcfc(x, &mut con);
        resmax = 0.0;
        k = 1;
        while (k <= m) {
            resmax = js_max(resmax, (-con[k]));
            k += 1;
        }

        // DATMAT columns contain constraint values, objective, and maximum
        // constraint violation for each simplex vertex.
        con[mp] = f;
        con[mpp] = resmax;
        skipVertexIdent = true;
        if (!ibrnch) {
            skipVertexIdent = false;
            i = 1;
            while (i <= mpp) {
                datmat[i][jdrop] = con[i];
                i += 1;
            }
            // Complete the initial simplex, exchanging its optimal vertex.
            if (nfvals <= np) {
                if (jdrop <= n) {
                    if (datmat[mp][np] <= f) {
                        x[jdrop] = sim[jdrop][np];
                    } else {
                        sim[jdrop][np] = x[jdrop];
                        k = 1;
                        while (k <= mpp) {
                            datmat[k][jdrop] = datmat[k][np];
                            datmat[k][np] = con[k];
                            k += 1;
                        }
                        k = 1;
                        while (k <= jdrop) {
                            sim[jdrop][k] = (-rho);
                            temp = 0.0;
                            i = k;
                            while (i <= jdrop) {
                                temp -= simi[i][k];
                                i += 1;
                            }
                            simi[jdrop][k] = temp;
                            k += 1;
                        }
                    }
                }
                if (nfvals <= n) {
                    jdrop = nfvals;
                    x[jdrop] += rho;
                    continue 'l_40;
                }
            }
            ibrnch = true;
        }
        // L140: identify the optimal vertex, then improve the current simplex.
        'l_140: loop {
            'l_550: {
                if (!skipVertexIdent) {
                    phimin = (datmat[mp][np] + (parmu * datmat[mpp][np]));
                    nbest = np;
                    j = 1;
                    while (j <= n) {
                        temp = (datmat[mp][j] + (parmu * datmat[mpp][j]));
                        if (temp < phimin) {
                            nbest = j;
                            phimin = temp;
                        } else {
                            if (((temp == phimin) && (parmu == 0.0))
                                && (datmat[mpp][j] < datmat[mpp][nbest]))
                            {
                                nbest = j;
                            }
                        }
                        j += 1;
                    }
                    // Move the optimal vertex into pole position.
                    if (nbest <= n) {
                        i = 1;
                        while (i <= mpp) {
                            temp = datmat[i][np];
                            datmat[i][np] = datmat[i][nbest];
                            datmat[i][nbest] = temp;
                            i += 1;
                        }
                        i = 1;
                        while (i <= n) {
                            temp = sim[i][nbest];
                            sim[i][nbest] = 0.0;
                            sim[i][np] += temp;
                            tempa = 0.0;
                            k = 1;
                            while (k <= n) {
                                sim[i][k] -= temp;
                                tempa -= simi[k][i];
                                k += 1;
                            }
                            simi[nbest][i] = tempa;
                            i += 1;
                        }
                    }
                    // Optional inverse-accuracy check; upstream public
                    // FindMinimum leaves this experimental option disabled.
                    error = 0.0;
                    if testForRoundingErrors {
                        i = 1;
                        while (i <= n) {
                            j = 1;
                            while (j <= n) {
                                temp = (dot_column(&simi[i], &sim, j, 1, n)
                                    - (if (i == j) { 1.0 } else { 0.0 }));
                                error = js_max(error, (temp).abs());
                                j += 1;
                            }
                            i += 1;
                        }
                    }
                    if (error > 0.1) {
                        status = 2;
                        break 'l_40;
                    }
                    // Linearize constraints and the negative objective gradient.
                    k = 1;
                    while (k <= mp) {
                        con[k] = (-datmat[k][np]);
                        j = 1;
                        while (j <= n) {
                            w[j] = (datmat[k][j] + con[k]);
                            j += 1;
                        }
                        i = 1;
                        while (i <= n) {
                            a[i][k] = ((if (k == mp) { (-1.0) } else { 1.0 })
                                * dot_column(&w, &simi, i, 1, n));
                            i += 1;
                        }
                        k += 1;
                    }
                    // Sigma and eta measure whether the simplex is acceptable.
                    iflag = true;
                    parsig = (alpha * rho);
                    pareta = (beta * rho);
                    j = 1;
                    while (j <= n) {
                        wsig = 0.0;
                        weta = 0.0;
                        k = 1;
                        while (k <= n) {
                            wsig += (simi[j][k] * simi[j][k]);
                            weta += (sim[k][j] * sim[k][j]);
                            k += 1;
                        }
                        vsig[j] = (1.0 / (wsig).sqrt());
                        veta[j] = (weta).sqrt();
                        if ((vsig[j] < parsig) || (veta[j] > pareta)) {
                            iflag = false;
                        }
                        j += 1;
                    }
                    // If necessary, choose a replacement vertex to improve shape.
                    if ((!ibrnch) && (!iflag)) {
                        jdrop = 0;
                        temp = pareta;
                        j = 1;
                        while (j <= n) {
                            if (veta[j] > temp) {
                                jdrop = j;
                                temp = veta[j];
                            }
                            j += 1;
                        }
                        if (jdrop == 0) {
                            j = 1;
                            while (j <= n) {
                                if (vsig[j] < temp) {
                                    jdrop = j;
                                    temp = vsig[j];
                                }
                                j += 1;
                            }
                        }
                        temp = ((gamma * rho) * vsig[jdrop]);
                        k = 1;
                        while (k <= n) {
                            dx[k] = (temp * simi[jdrop][k]);
                            k += 1;
                        }
                        cvmaxp = 0.0;
                        cvmaxm = 0.0;
                        total = 0.0;
                        k = 1;
                        while (k <= mp) {
                            total = dot_column(&dx, &a, k, 1, n);
                            if (k < mp) {
                                temp = datmat[k][np];
                                cvmaxp = js_max(cvmaxp, ((-total) - temp));
                                cvmaxm = js_max(cvmaxm, (total - temp));
                            }
                            k += 1;
                        }
                        dxsign = (if ((parmu * (cvmaxp - cvmaxm)) > (2.0 * total)) {
                            (-1.0)
                        } else {
                            1.0
                        });
                        temp = 0.0;
                        i = 1;
                        while (i <= n) {
                            dx[i] = (dxsign * dx[i]);
                            sim[i][jdrop] = dx[i];
                            temp += (simi[jdrop][i] * dx[i]);
                            i += 1;
                        }
                        k = 1;
                        while (k <= n) {
                            simi[jdrop][k] /= temp;
                            k += 1;
                        }
                        j = 1;
                        while (j <= n) {
                            if (j != jdrop) {
                                temp = dot(&simi[j], &dx, 1, n);
                                k = 1;
                                while (k <= n) {
                                    {
                                        let update = (temp * simi[jdrop][k]);
                                        simi[j][k] -= update;
                                    };
                                    k += 1;
                                }
                            }
                            x[j] = (sim[j][np] + dx[j]);
                            j += 1;
                        }
                        continue 'l_40;
                    }
                    // Solve the linearized trust-region subproblem.
                    ifull = trust_region::trstlp(n, m, &a, &con, rho, &mut dx);
                    if (!ifull) {
                        temp = 0.0;
                        k = 1;
                        while (k <= n) {
                            temp += (dx[k] * dx[k]);
                            k += 1;
                        }
                        if (temp < ((0.25 * rho) * rho)) {
                            ibrnch = true;
                            break 'l_550;
                        }
                    }
                    total = 0.0;
                    resnew = 0.0;
                    con[mp] = 0.0;
                    k = 1;
                    while (k <= mp) {
                        total = (con[k] - dot_column(&dx, &a, k, 1, n));
                        if (k < mp) {
                            resnew = js_max(resnew, total);
                        }
                        k += 1;
                    }
                    // Increase the constraint penalty only when the predicted
                    // merit reduction requires it.
                    prerec = (datmat[mpp][np] - resnew);
                    barmu = (if (prerec > 0.0) {
                        (total / prerec)
                    } else {
                        0.0
                    });
                    if (parmu < (1.5 * barmu)) {
                        parmu = (2.0 * barmu);

                        phi = (datmat[mp][np] + (parmu * datmat[mpp][np]));
                        j = 1;
                        while (j <= n) {
                            temp = (datmat[mp][j] + (parmu * datmat[mpp][j]));
                            if ((temp < phi)
                                || (((temp == phi) && (parmu == 0.0))
                                    && (datmat[mpp][j] < datmat[mpp][np])))
                            {
                                continue 'l_140;
                            }
                            j += 1;
                        }
                    }
                    prerem = ((parmu * prerec) - total);
                    k = 1;
                    while (k <= n) {
                        x[k] = (sim[k][np] + dx[k]);
                        k += 1;
                    }
                    ibrnch = true;
                    continue 'l_40;
                }
                // Compare actual and predicted merit reduction at the trial.
                skipVertexIdent = false;
                vmold = (datmat[mp][np] + (parmu * datmat[mpp][np]));
                vmnew = (f + (parmu * resmax));
                trured = (vmold - vmnew);
                if ((parmu == 0.0) && (f == datmat[mp][np])) {
                    prerem = prerec;
                    trured = (datmat[mpp][np] - resmax);
                }
                // Choose the simplex vertex replaced by the evaluated point.
                ratio = (if (trured <= 0.0) { 1.0 } else { 0.0 });
                jdrop = 0;
                j = 1;
                while (j <= n) {
                    temp = (dot(&simi[j], &dx, 1, n)).abs();
                    if (temp > ratio) {
                        jdrop = j;
                        ratio = temp;
                    }
                    sigbar[j] = (temp * vsig[j]);
                    j += 1;
                }
                edgmax = (delta * rho);
                l = 0;
                j = 1;
                while (j <= n) {
                    if ((sigbar[j] >= parsig) || (sigbar[j] >= vsig[j])) {
                        temp = veta[j];
                        if (trured > 0.0) {
                            temp = 0.0;
                            k = 1;
                            while (k <= n) {
                                temp += (dx[k] - sim[k][j]).powi(2);
                                k += 1;
                            }
                            temp = (temp).sqrt();
                        }
                        if (temp > edgmax) {
                            l = j;
                            edgmax = temp;
                        }
                    }
                    j += 1;
                }
                if (l > 0) {
                    jdrop = l;
                }
                // Update the simplex and inverse without reordering arithmetic.
                if (jdrop != 0) {
                    temp = 0.0;
                    i = 1;
                    while (i <= n) {
                        sim[i][jdrop] = dx[i];
                        temp += (simi[jdrop][i] * dx[i]);
                        i += 1;
                    }
                    k = 1;
                    while (k <= n) {
                        simi[jdrop][k] /= temp;
                        k += 1;
                    }
                    j = 1;
                    while (j <= n) {
                        if (j != jdrop) {
                            temp = dot(&simi[j], &dx, 1, n);
                            k = 1;
                            while (k <= n) {
                                {
                                    let update = (temp * simi[jdrop][k]);
                                    simi[j][k] -= update;
                                };
                                k += 1;
                            }
                        }
                        j += 1;
                    }
                    k = 1;
                    while (k <= mpp) {
                        datmat[k][jdrop] = con[k];
                        k += 1;
                    }
                    if ((trured > 0.0) && (trured >= (0.1 * prerem))) {
                        continue 'l_140;
                    }
                }
            }
            if (!iflag) {
                ibrnch = false;
                continue 'l_140;
            }
            if (rho <= rhoend) {
                status = 0;
                break 'l_40;
            }
            // Reduce the radius and reset the penalty for the next stage.
            cmin = 0.0;
            cmax = 0.0;
            rho *= 0.5;
            if (rho <= (1.5 * rhoend)) {
                rho = rhoend;
            }
            if (parmu > 0.0) {
                denom = 0.0;
                k = 1;
                while (k <= mp) {
                    cmin = datmat[k][np];
                    cmax = cmin;
                    i = 1;
                    while (i <= n) {
                        cmin = js_min(cmin, datmat[k][i]);
                        cmax = js_max(cmax, datmat[k][i]);
                        i += 1;
                    }
                    if ((k <= m) && (cmin < (0.5 * cmax))) {
                        temp = (js_max(cmax, 0.0) - cmin);
                        denom = (if (denom <= 0.0) {
                            temp
                        } else {
                            js_min(denom, temp)
                        });
                    }
                    k += 1;
                }
                if (denom == 0.0) {
                    parmu = 0.0;
                } else {
                    if ((cmax - cmin) < (parmu * denom)) {
                        parmu = ((cmax - cmin) / denom);
                    }
                }
            }
        }
    }
    // On a full normal step preserve the current point. Otherwise restore the
    // pole vertex, including evaluation-budget and rounding-error exits.
    if status == 0 && ifull {
        return (status, nfvals);
    }
    k = 1;
    while (k <= n) {
        x[k] = sim[k][np];
        k += 1;
    }
    f = datmat[mp][np];
    resmax = datmat[mpp][np];

    (status, nfvals)
}

fn dot(a: &[f64], b: &[f64], start: usize, end: usize) -> f64 {
    let mut sum = 0.0;
    for i in start..=end {
        sum += a[i] * b[i];
    }
    sum
}
fn dot_column(a: &[f64], b: &[Vec<f64>], col: usize, start: usize, end: usize) -> f64 {
    let mut sum = 0.0;
    for i in start..=end {
        sum += a[i] * b[i][col];
    }
    sum
}
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_positive() || b.is_sign_positive() {
            0.0
        } else {
            -0.0
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
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() || b.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else if a < b {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    // The upstream callback values are replayed rather than recomputed with
    // Rust transcendental functions: this isolates solver iteration parity
    // from the host's Math.hypot/atan2 implementations. Every evaluated point,
    // the evaluation count, exit status, and selected final iterate are checked.
    #[test]
    fn actual_upstream_evaluation_traces_match() {
        let fixture: Value =
            serde_json::from_str(include_str!("../tests/fixtures/upstream-cobyla.json")).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut x = numbers(&case["initial"]);
            let samples = case["samples"].as_array().unwrap();
            let mut evaluation = 0;
            let outcome = find_minimum(
                |point, con| {
                    let sample = samples.get(evaluation).unwrap_or_else(|| {
                        panic!("{name}: extra evaluation {evaluation}: {point:?}")
                    });
                    close(point, &numbers(&sample["x"]), name, evaluation);
                    con.copy_from_slice(&numbers(&sample["con"]));
                    evaluation += 1;
                    sample["f"].as_f64().unwrap()
                },
                case["constraints"].as_u64().unwrap() as usize,
                &mut x,
                case["rhobeg"].as_f64().unwrap(),
                case["rhoend"].as_f64().unwrap(),
                case["maxfun"].as_u64().unwrap() as usize,
            );
            close(&x, &numbers(&case["result"]), name, evaluation);
            assert_eq!(evaluation, samples.len(), "{name}");
            assert_eq!(outcome.evaluations, evaluation, "{name}");
            let status = match outcome.status {
                Status::Normal => 0,
                Status::MaxEvaluations => 1,
                Status::RoundingErrors => 2,
            };
            assert_eq!(status, case["status"].as_i64().unwrap(), "{name}");
        }
    }

    #[test]
    fn inequality_and_equality_callbacks_receive_original_parameter_units() {
        let mut bounded = [100.];
        let out = find_minimum(
            |x, c| {
                c[0] = x[0];
                c[1] = 240. - x[0];
                (x[0] - 300.).powi(2)
            },
            2,
            &mut bounded,
            200.,
            1e-3,
            200,
        );
        assert_eq!(out.status, Status::Normal);
        assert!((bounded[0] - 240.).abs() < 1e-6);
        let mut line = [20., 10.];
        let out = find_minimum(
            |x, c| {
                c[0] = x[0] + x[1] - 60.;
                c[1] = -c[0];
                (x[0] - 120.).powi(2) + (x[1] - 85.).powi(2)
            },
            2,
            &mut line,
            100.,
            1e-3,
            200,
        );
        assert_eq!(out.status, Status::Normal);
        assert!((line[0] + line[1] - 60.).abs() < 1e-10);
        assert!((line[0] - 47.5).abs() < 0.01);
    }

    fn numbers(value: &Value) -> Vec<f64> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }
    fn close(actual: &[f64], expected: &[f64], name: &str, evaluation: usize) {
        assert_eq!(actual.len(), expected.len(), "{name}");
        for (i, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            let tolerance = 0.;
            assert!(
                (actual - expected).abs() <= tolerance,
                "{name}, evaluation {evaluation}, coordinate {i}: Rust {actual:.17e}, source {expected:.17e}"
            );
        }
    }
}
