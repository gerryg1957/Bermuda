use crate::{Board, Color};

pub fn render(board: &Board) -> String {
    let mut output = String::new();

    output.push_str("   ");
    for x in 0..board.size() {
        output.push(column_label(x));
        output.push(' ');
    }
    output.push('\n');

    for y in (0..board.size()).rev() {
        output.push_str(&format!("{:>2} ", y + 1));

        for x in 0..board.size() {
            let point = board.point(x, y).unwrap();

            let symbol = match board.color_at(point) {
                Some(Color::Black) => 'X',
                Some(Color::White) => 'O',
                None => '.',
            };

            output.push(symbol);
            output.push(' ');
        }

        output.push('\n');
    }

    output
}

fn column_label(x: u8) -> char {
    let adjusted = if x >= 8 { x + 1 } else { x };
    char::from(b'A' + adjusted)
}
