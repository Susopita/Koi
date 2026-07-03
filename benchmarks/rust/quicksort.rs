fn quicksort(arr: &mut [i32]) {
    let len = arr.len();
    if len <= 1 {
        return;
    }
    quicksort_range(arr, 0, len as isize - 1);
}

fn quicksort_range(arr: &mut [i32], low: isize, high: isize) {
    if low < high {
        let p = partition(arr, low, high);
        quicksort_range(arr, low, p - 1);
        quicksort_range(arr, p + 1, high);
    }
}

fn partition(arr: &mut [i32], low: isize, high: isize) -> isize {
    let pivot = arr[high as usize];
    let mut i = low - 1;
    for j in low..high {
        if arr[j as usize] <= pivot {
            i += 1;
            arr.swap(i as usize, j as usize);
        }
    }
    arr.swap((i + 1) as usize, high as usize);
    i + 1
}

// Generador congruencial lineal simple para tener datos deterministas
// sin depender de crates externos (rand no está en std).
fn lcg_next(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

fn main() {
    let n = 100_000;
    let mut state: u64 = 88172645463325252;
    let mut data: Vec<i32> = Vec::with_capacity(n);
    for _ in 0..n {
        data.push((lcg_next(&mut state) % 1_000_000) as i32);
    }

    quicksort(&mut data);

    // Tocar el resultado para que el compilador no elimine el trabajo.
    println!("{} {}", data[0], data[n - 1]);
}
