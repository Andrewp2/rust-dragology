// Initial/goal port graphs transcribed from the MIT upstream demos.

fn preset(tangle: bool, i: usize) -> State {
    if tangle {
        return match i {
            0 => {
                let mut s = State::new(true, 0);
                s.add("a", Kind::Input, 22.0, 120.0);
                s.add("b", Kind::Input, 22.0, 240.0);
                s.add("c", Kind::Output, 438.0, 120.0);
                s.add("e", Kind::Output, 438.0, 240.0);
                s.add("d", Kind::Comult, 160.0, 120.0);
                s.add("m", Kind::Mult, 300.0, 240.0);
                s.wire("a", 0, "d", 0);
                s.wire("d", 0, "c", 0);
                s.wire("d", 0, "m", 0);
                s.wire("b", 0, "m", 0);
                s.wire("m", 0, "e", 0);
                assign_ports(s)
            }
            1 => {
                let mut s = State::new(true, 1);
                s.add("in1", Kind::Input, 22.0, 100.0);
                s.add("in2", Kind::Input, 22.0, 260.0);
                s.add("out1", Kind::Output, 438.0, 100.0);
                s.add("out2", Kind::Output, 438.0, 260.0);
                s.add("d1", Kind::Comult, 140.0, 100.0);
                s.add("d2", Kind::Comult, 140.0, 260.0);
                s.add("m1", Kind::Mult, 320.0, 100.0);
                s.add("m2", Kind::Mult, 320.0, 260.0);
                s.wire("in1", 0, "d1", 0);
                s.wire("in2", 0, "d2", 0);
                s.wire("d1", 0, "m1", 0);
                s.wire("d1", 0, "m2", 0);
                s.wire("d2", 0, "m1", 0);
                s.wire("d2", 0, "m2", 0);
                s.wire("m1", 0, "out1", 0);
                s.wire("m2", 0, "out2", 0);
                assign_ports(s)
            }
            2 => {
                let mut s = State::new(true, 2);
                s.add("in1", Kind::Input, 22.0, 180.0);
                s.add("out1", Kind::Output, 438.0, 180.0);
                s.add("d1", Kind::Comult, 100.0, 180.0);
                s.add("d2", Kind::Comult, 180.0, 100.0);
                s.add("d3", Kind::Comult, 180.0, 260.0);
                s.add("m1", Kind::Mult, 280.0, 100.0);
                s.add("m2", Kind::Mult, 280.0, 260.0);
                s.add("m3", Kind::Mult, 360.0, 180.0);
                s.wire("in1", 0, "d1", 0);
                s.wire("d1", 0, "d2", 0);
                s.wire("d1", 0, "d3", 0);
                s.wire("d2", 0, "m1", 0);
                s.wire("d2", 0, "m2", 0);
                s.wire("d3", 0, "m1", 0);
                s.wire("d3", 0, "m2", 0);
                s.wire("m1", 0, "m3", 0);
                s.wire("m2", 0, "m3", 0);
                s.wire("m3", 0, "out1", 0);
                assign_ports(s)
            }
            3 => {
                let mut s = State::new(true, 3);
                s.add("in1", Kind::Input, 22.0, 157.91);
                s.add("in2", Kind::Input, 22.0, 221.09);
                s.add("out1", Kind::Output, 438.0, 190.715);
                s.add("d1", Kind::Comult, 104.62, 138.47);
                s.add("d2", Kind::Comult, 104.62, 240.53);
                s.add("d3", Kind::Comult, 162.94, 88.655);
                s.add("d4", Kind::Comult, 162.94, 290.345);
                s.add("m1", Kind::Mult, 240.7, 87.44);
                s.add("m2", Kind::Mult, 240.7, 292.775);
                s.add("m3", Kind::Mult, 299.02, 134.825);
                s.add("m4", Kind::Mult, 299.02, 244.17499999999998);
                s.add("m5", Kind::Mult, 371.92, 190.715);
                s.wire("in1", 0, "d1", 0);
                s.wire("in2", 0, "d2", 0);
                s.wire("d1", 0, "d3", 0);
                s.wire("d1", 0, "m2", 0);
                s.wire("d2", 0, "d4", 0);
                s.wire("d2", 0, "m1", 0);
                s.wire("d3", 0, "m1", 0);
                s.wire("d3", 0, "m4", 0);
                s.wire("d4", 0, "m2", 0);
                s.wire("d4", 0, "m3", 0);
                s.wire("m1", 0, "m3", 0);
                s.wire("m2", 0, "m4", 0);
                s.wire("m3", 0, "m5", 0);
                s.wire("m4", 0, "m5", 0);
                s.wire("m5", 0, "out1", 0);
                assign_ports(s)
            }
            _ => tangle_braid(),
        };
    }
    match i {
        0 => {
            let mut s = State::new(false, 0);
            s.add("a", Kind::Input, 130., 0.);
            s.add("b", Kind::Input, 230., 0.);
            s.add("out", Kind::Output, 160., 0.);
            s.add("eta", Kind::Unit, 70., 1.);
            s.add("u", Kind::Mult, 100., 2.);
            s.add("l", Kind::Mult, 160., 3.);
            s.wire("eta", 0, "u", 0);
            s.wire("a", 0, "u", 1);
            s.wire("u", 0, "l", 0);
            s.wire("b", 0, "l", 1);
            s.wire("l", 0, "out", 0);
            s.tidy(None)
        }
        1 => {
            let mut s = State::new(false, 1);
            s.add("a", Kind::Input, 70., 0.);
            s.add("b", Kind::Input, 140., 0.);
            s.add("c", Kind::Input, 250., 0.);
            s.add("o1", Kind::Output, 70., 0.);
            s.add("o2", Kind::Output, 140., 0.);
            s.add("o3", Kind::Output, 250., 0.);
            s.add("u", Kind::Mult, 105., 1.);
            s.add("l", Kind::Mult, 160., 2.);
            s.add("d1", Kind::Comult, 160., 3.);
            s.add("d2", Kind::Comult, 105., 4.);
            s.wire("a", 0, "u", 0);
            s.wire("b", 0, "u", 1);
            s.wire("u", 0, "l", 0);
            s.wire("c", 0, "l", 1);
            s.wire("l", 0, "d1", 0);
            s.wire("d1", 0, "d2", 0);
            s.wire("d1", 1, "o3", 0);
            s.wire("d2", 0, "o1", 0);
            s.wire("d2", 1, "o2", 0);
            s.tidy(None)
        }
        2 => {
            let mut s = State::new(false, 2);
            s.add("a", Kind::Input, 110., 0.);
            s.add("b", Kind::Input, 210., 0.);
            s.add("c", Kind::Output, 110., 0.);
            s.add("d", Kind::Output, 210., 0.);
            s.add("m", Kind::Mult, 160., 1.);
            s.add("k", Kind::Comult, 160., 2.);
            s.wire("a", 0, "m", 0);
            s.wire("b", 0, "m", 1);
            s.wire("m", 0, "k", 0);
            s.wire("k", 0, "c", 0);
            s.wire("k", 1, "d", 0);
            s.tidy(None)
        }
        3 => {
            let mut s = State::new(false, 3);
            s.add("a", Kind::Input, 90., 0.);
            s.add("out", Kind::Output, 230., 0.);
            s.add("eta", Kind::Unit, 230., 1.);
            s.add("k", Kind::Comult, 230., 2.);
            s.add("m", Kind::Mult, 160., 3.);
            s.add("eps", Kind::Counit, 160., 4.);
            s.wire("eta", 0, "k", 0);
            s.wire("a", 0, "m", 0);
            s.wire("k", 0, "m", 1);
            s.wire("m", 0, "eps", 0);
            s.wire("k", 1, "out", 0);
            s.tidy(None)
        }
        4 => {
            let mut s = State::new(false, 4);
            s.add("a", Kind::Input, 90., 0.);
            s.add("out", Kind::Output, 90., 0.);
            s.add("eta1", Kind::Unit, 230., 1.);
            s.add("k1", Kind::Comult, 230., 2.);
            s.add("m1", Kind::Mult, 160., 3.);
            s.add("eps1", Kind::Counit, 160., 4.);
            s.add("eta2", Kind::Unit, 90., 5.);
            s.add("k2", Kind::Comult, 90., 6.);
            s.add("m2", Kind::Mult, 160., 7.);
            s.add("eps2", Kind::Counit, 160., 8.);
            s.wire("eta1", 0, "k1", 0);
            s.wire("a", 0, "m1", 0);
            s.wire("k1", 0, "m1", 1);
            s.wire("m1", 0, "eps1", 0);
            s.wire("eta2", 0, "k2", 0);
            s.wire("k2", 1, "m2", 0);
            s.wire("k1", 1, "m2", 1);
            s.wire("m2", 0, "eps2", 0);
            s.wire("k2", 0, "out", 0);
            s.tidy(None)
        }
        5 => {
            let mut s = State::new(false, 5);
            s.add("a", Kind::Input, 60., 0.);
            s.add("b", Kind::Input, 130., 0.);
            s.add("c", Kind::Input, 260., 0.);
            s.add("o1", Kind::Output, 60., 0.);
            s.add("o2", Kind::Output, 160., 0.);
            s.add("o3", Kind::Output, 260., 0.);
            s.add("u", Kind::Mult, 95., 1.);
            s.add("eta", Kind::Unit, 230., 2.);
            s.add("l", Kind::Mult, 130., 3.);
            s.add("k", Kind::Comult, 130., 4.);
            s.add("k2", Kind::Comult, 230., 5.);
            s.add("eps", Kind::Counit, 200., 6.);
            s.wire("a", 0, "u", 0);
            s.wire("b", 0, "u", 1);
            s.wire("u", 0, "l", 0);
            s.wire("c", 0, "k2", 0);
            s.wire("k2", 0, "eps", 0);
            s.wire("k2", 1, "o3", 0);
            s.wire("eta", 0, "l", 1);
            s.wire("l", 0, "k", 0);
            s.wire("k", 0, "o1", 0);
            s.wire("k", 1, "o2", 0);
            s.tidy(None)
        }
        _ => unreachable!(),
    }
}

fn goal(s: &State) -> Option<State> {
    if s.tangle {
        return Some(normal(s));
    }
    Some(match s.puzzle {
        0 => {
            let mut s = State::new(false, 0);
            s.add("a", Kind::Input, 110., 0.);
            s.add("b", Kind::Input, 210., 0.);
            s.add("out", Kind::Output, 160., 0.);
            s.add("m", Kind::Mult, 160., 1.);
            s.wire("a", 0, "m", 0);
            s.wire("b", 0, "m", 1);
            s.wire("m", 0, "out", 0);
            s.tidy(None)
        }
        1 => {
            let mut s = State::new(false, 1);
            s.add("a", Kind::Input, 70., 0.);
            s.add("b", Kind::Input, 180., 0.);
            s.add("c", Kind::Input, 250., 0.);
            s.add("o1", Kind::Output, 70., 0.);
            s.add("o2", Kind::Output, 180., 0.);
            s.add("o3", Kind::Output, 250., 0.);
            s.add("u", Kind::Mult, 215., 1.);
            s.add("l", Kind::Mult, 160., 2.);
            s.add("d1", Kind::Comult, 160., 3.);
            s.add("d2", Kind::Comult, 215., 4.);
            s.wire("a", 0, "l", 0);
            s.wire("b", 0, "u", 0);
            s.wire("c", 0, "u", 1);
            s.wire("u", 0, "l", 1);
            s.wire("l", 0, "d1", 0);
            s.wire("d1", 0, "o1", 0);
            s.wire("d1", 1, "d2", 0);
            s.wire("d2", 0, "o2", 0);
            s.wire("d2", 1, "o3", 0);
            s.tidy(None)
        }
        2 => {
            let mut s = State::new(false, 2);
            s.add("a", Kind::Input, 110., 0.);
            s.add("b", Kind::Input, 210., 0.);
            s.add("c", Kind::Output, 110., 0.);
            s.add("d", Kind::Output, 210., 0.);
            s.add("k", Kind::Comult, 110., 1.);
            s.add("m", Kind::Mult, 210., 2.);
            s.wire("a", 0, "k", 0);
            s.wire("k", 0, "c", 0);
            s.wire("k", 1, "m", 0);
            s.wire("b", 0, "m", 1);
            s.wire("m", 0, "d", 0);
            s.tidy(None)
        }
        3 | 4 => {
            let mut t = State::new(false, s.puzzle);
            t.add("a", Kind::Input, 160., 0.);
            t.add("out", Kind::Output, 160., 0.);
            t.wire("a", 0, "out", 0);
            t
        }
        _ => return None,
    })
}

fn assign_ports(mut s: State) -> State {
    for id in s.nodes.keys().cloned().collect::<Vec<_>>() {
        for outgoing in [false, true] {
            let mut ids: Vec<_> = s
                .wires
                .iter()
                .filter(|(_, w)| {
                    if outgoing {
                        w.from.node == id
                    } else {
                        w.to.node == id
                    }
                })
                .map(|(i, w)| {
                    (
                        *i,
                        s.pos(if outgoing { &w.to.node } else { &w.from.node }).y,
                    )
                })
                .collect();
            ids.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            for (port, (wid, _)) in ids.into_iter().enumerate() {
                let w = s.wires.get_mut(&wid).unwrap();
                if outgoing {
                    w.from.i = port
                } else {
                    w.to.i = port
                }
            }
        }
    }
    s
}
fn tangle_braid() -> State {
    let mut s = State::new(true, 4);
    for (i, y) in [90., 180., 270.].into_iter().enumerate() {
        s.add(&format!("in{i}"), Kind::Input, 22., y);
        s.add(&format!("out{i}"), Kind::Output, 438., y);
    }
    let mut feeds = (0..3).map(|i| format!("in{i}")).collect::<Vec<_>>();
    for (round, x) in [80., 230.].into_iter().enumerate() {
        for (i, y) in [90., 180., 270.].into_iter().enumerate() {
            s.add(&format!("d{round}{i}"), Kind::Comult, x, y);
            s.add(&format!("m{round}{i}"), Kind::Mult, x + 80., y);
            s.wire(&feeds[i], 0, &format!("d{round}{i}"), 0);
        }
        for i in 0..3 {
            s.wire(&format!("d{round}{i}"), 0, &format!("m{round}{i}"), 0);
            s.wire(
                &format!("d{round}{}", (i + 1) % 3),
                0,
                &format!("m{round}{i}"),
                0,
            );
        }
        feeds = (0..3).map(|i| format!("m{round}{i}")).collect();
    }
    for (i, feed) in feeds.iter().enumerate() {
        s.wire(feed, 0, &format!("out{i}"), 0);
    }
    assign_ports(s)
}
