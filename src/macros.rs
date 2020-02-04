macro_rules! lockprintln {
    () => (print!("\n"));
    ($($arg:tt)*) => ({
        use std::io::Write;
        std::writeln!(std::io::stdout().lock(), $($arg)*).expect("LPRINTLN");
    })
}
macro_rules! assert_let {
    ($pat:pat = $expr:expr => $work:expr) => {
        if let $pat = $expr {
            $work
        } else {
            panic!("assert_let failed");
        }
    };
}

#[test]
fn assert_let() {
    let x = Some(5);
    let z = assert_let![Some(y) = x => {
        println!("{:?}", y);
        3
    }];
    println!("{:?}", z);
}

#[test]
#[should_panic]
fn must_let_panic() {
    let x: Option<u32> = None;
    assert_let![Some(y) = x => {
        println!("{:?}", y);
    }];
}
