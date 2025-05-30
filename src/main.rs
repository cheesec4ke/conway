use ahash::AHashMap;
use clap::Parser;
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, execute, queue, terminal};
use std::io::{Stdout, Write};
use std::time::{Duration, Instant};
use std::{io, thread};

///Formats and prints each argument with a new line to a given writer
macro_rules! printnl {
    ($writer:expr, $($text:expr),+ $(,)?) => {{
        $(
            execute!(
                $writer,
                Print(format!($text)),
                Clear(ClearType::UntilNewLine),
                cursor::MoveToNextLine(1)
            ).unwrap();
        )+
    }};
}

#[derive(Parser)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    fps: Option<u64>,
    #[arg(short = 'x', long)]
    width: Option<usize>,
    #[arg(short = 'y', long)]
    height: Option<usize>,
    #[arg(
        short,
        long,
        require_equals = true,
        default_value = "false",
        default_missing_value = "true",
        num_args = 0..1,
    )]
    quiet: bool,
}

fn main() {
    let args = Args::parse();

    //set board size according to terminal size with some extra space at the bottom unless set otherwise
    let mut blank_lines: u16 = 7;
    if args.quiet {
        blank_lines = 3;
    }
    let mut width = terminal::size().unwrap().0 as usize;
    if let Some(w) = args.width {
        width = w;
    }
    let mut height = (
        (terminal::size().unwrap().1 * 2) - (blank_lines * 2)
    ) as usize;
    if let Some(h) = args.height {
        height = h;
    }

    //initialize variables
    let mut board = random_board(width, height);
    let mut generation = 0usize;
    let mut board_history = AHashMap::new();
    let mut stdout = io::stdout();

    let frame_time: Duration;
    match args.fps {
        None => frame_time = get_fps(20),
        Some(0) => frame_time = Duration::ZERO,
        Some(fps) => frame_time = Duration::from_millis(1000 / fps)
    }

    queue!(stdout, Clear(ClearType::All), cursor::Hide).unwrap(); //prep console

    //main loop
    let start_time = Instant::now(); //start counting for average fps calculation
    let mut deadline = Instant::now() + frame_time; //set the fps limit
    loop {
        queue!(stdout, cursor::MoveTo(0, 0)).unwrap();
        print_board(&board, &mut stdout);
        if !args.quiet {
            print_stats(&board, generation, &mut stdout);
        }
        match detect_loop(&board, generation, &mut board_history) {
            Some(loop_start) => {
                if !args.quiet {
                    printnl!(stdout, "Game started looping from generation {loop_start}");
                }
                break;
            }
            None => {
                iterate_board(&mut board);
                generation += 1;
            }
        }
        if !frame_time.is_zero() {
            sleep_until(deadline);
            deadline += frame_time;
        };
    }

    //print average fps if unlimited
    if frame_time.is_zero() && !args.quiet {
        execute!(
            stdout,
            Print(format!(
                "FPS: ~{:.2}",
                generation as f64 / start_time.elapsed().as_secs_f64()
            )),
            Clear(ClearType::UntilNewLine),
        ).unwrap();
    }

    //cleanup
    if !args.quiet {
        execute!(stdout, cursor::MoveToNextLine(1)).unwrap();
    }
    execute!(stdout, cursor::Show).unwrap();
}

///Generates a random board with a given width and height
fn random_board(width: usize, height: usize) -> Vec<Vec<bool>> {
    let mut board = vec![vec![false; width]; height];
    for x in 0..width {
        for y in 0..height {
            if fastrand::bool() {
                board[y][x] = true;
            }
        }
    }
    board
}

///Asks the user for their desired FPS, uses default_fps if their input is invalid
fn get_fps(default_fps: u64) -> Duration {
    print!("Input the desired FPS (0 for unlimited, default {default_fps}): ");
    io::stdout().flush().unwrap();
    let mut input_text = String::new();
    io::stdin().read_line(&mut input_text).unwrap();
    let target_fps = input_text.trim().parse::<u64>().unwrap_or(default_fps);
    let frame_time: Duration;
    if target_fps > 0 {
        frame_time = Duration::from_millis(1000 / target_fps);
    } else {
        frame_time = Duration::from_millis(0);
    }
    frame_time
}

const NEIGHBOR_OFFSETS: [(isize, isize); 8] = [
    (-1, -1), (-1, 0), (-1, 1),
    ( 0, -1),          ( 0, 1),
    ( 1, -1), ( 1, 0), ( 1, 1),
];

///Takes a board and iterates it according to the rules of the game
fn iterate_board(board: &mut Vec<Vec<bool>>) {
    let height = board.len();
    let width = board[0].len();

    //create empty board to be populated with surviving cells
    let mut new_board = vec![vec![false; width]; height];

    for y in 0..height {
        for x in 0..width {
            let mut neighbors = 0;
            //check neighboring cells
            for (y_offset, x_offset) in NEIGHBOR_OFFSETS.iter() {
                let y_pos = y as isize + y_offset;
                let x_pos = x as isize + x_offset;
                if y_pos >= 0
                    && x_pos >= 0
                    && y_pos < height as isize
                    && x_pos < width as isize
                    && board[y_pos as usize][x_pos as usize]
                {
                    neighbors += 1;
                }
            }
            //check if cell lives
            if board[y][x] && neighbors > 1 && neighbors < 4 {
                new_board[y][x] = true;
            } else if !board[y][x] && neighbors == 3 {
                new_board[y][x] = true;
            }
        }
    }
    board.clone_from(&new_board);
}

///Prints a board to stdout
fn print_board(board: &Vec<Vec<bool>>, stdout: &mut Stdout) {
    let mut buffer = String::with_capacity(board.len() * (board[0].len()));
    for y in 0..board.len() {
        //only add to buffer every 2 rows
        if y % 2 == 0 {
            for x in 0..board[0].len() {
                let pixels: char;
                //check if bottom pixel would be out of bounds
                if y + 1 != board.len() {
                    let character = (board[y][x], board[y + 1][x]);
                    match character {
                        (true, true) => pixels = '█',
                        (true, false) => pixels = '▀',
                        (false, true) => pixels = '▄',
                        (false, false) => pixels = ' ',
                    }
                } else {
                    match board[y][x] {
                        true => pixels = '▀',
                        false => pixels = ' ',
                    }
                }
                buffer.push(pixels);
            }
            buffer.push('\n');
        }
    }
    execute!(stdout, Print(buffer)).unwrap();
}

///Prints board statistics
fn print_stats(board: &Vec<Vec<bool>>, generation: usize, stdout: &mut Stdout) {
    let mut population = 0u32;
    for y in 0..board.len() {
        for x in 0..board[0].len() {
            if board[y][x] {
                population += 1;
            }
        }
    }
    printnl!(
        stdout,
        "Generation: {generation}",
        "Population: {population}"
    );
}

///Detects looping via hashmap of previous board states and returns the generation the loop started
fn detect_loop(
    board: &Vec<Vec<bool>>,
    generation: usize,
    history: &mut AHashMap<Vec<bool>, usize>,
) -> Option<usize> {
    history
        .insert(board.iter().flatten().cloned().collect(), generation)
        .and_then(Some)
}

fn sleep_until(deadline: Instant) {
    let now = Instant::now();

    if let Some(delay) = deadline.checked_duration_since(now) {
        thread::sleep(delay);
    }
}
