const N: usize = 500;

fn main() {
    let mut matrix = vec![vec![0i64; N]; N];

    for i in 0..N {
        for j in 0..N {
            matrix[i][j] = (i * N + j) as i64;
        }
    }

    let mut sum: i64 = 0;
    for i in 0..N {
        for j in 0..N {
            sum += matrix[i][j];
        }
    }

    println!("{}", sum);
}
