pub fn last_segment(s: &str) -> &str {
    s.rsplit('/').next().unwrap_or(s)
}

pub fn print_table(headers: &[&str; 5], rows: &[[String; 5]]) {
    let mut widths = [0usize; 5];
    for (i, h) in headers.iter().enumerate() {
        widths[i] = widths[i].max(display_width(h));
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(cell));
        }
    }

    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            print!("  ");
        }
        print!("{:width$}", h, width = widths[i]);
    }
    println!();

    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            print!("  ");
        }
        print!("{}", "-".repeat(*w));
    }
    println!();

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                print!("  ");
            }
            print!("{:width$}", cell, width = widths[i]);
        }
        println!();
    }
}

fn display_width(s: &str) -> usize {
    s.chars().count()
}
