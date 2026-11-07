//! Number formatting for incremental games (1.5K, 2.3M, 1.1B, etc.).

pub fn format_number(n: f64) -> String {
    if n < 1000.0 {
        return format!("{:.0}", n);
    }
    let suffixes = [
        "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", 
        "Dc", "Ud", "Dd", "Td", "Qad", "Qid", "Sxd", "Spd", "Ocd", "Nod", "Vg"
    ];
    let mut val = n;
    let mut i = 0;
    while val >= 1000.0 && i < suffixes.len() - 1 {
        val /= 1000.0;
        i += 1;
    }
    format!("{:.2}{}", val, suffixes[i - 1])
}