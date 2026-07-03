package main

import "fmt"

const N = 500

func main() {
	matrix := make([][]int64, N)
	for i := range matrix {
		matrix[i] = make([]int64, N)
	}

	for i := 0; i < N; i++ {
		for j := 0; j < N; j++ {
			matrix[i][j] = int64(i*N + j)
		}
	}

	var sum int64 = 0
	for i := 0; i < N; i++ {
		for j := 0; j < N; j++ {
			sum += matrix[i][j]
		}
	}

	fmt.Println(sum)
}
