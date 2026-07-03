package main

import "fmt"

func partition(arr []int32, low, high int) int {
	pivot := arr[high]
	i := low - 1
	for j := low; j < high; j++ {
		if arr[j] <= pivot {
			i++
			arr[i], arr[j] = arr[j], arr[i]
		}
	}
	arr[i+1], arr[high] = arr[high], arr[i+1]
	return i + 1
}

func quicksortRange(arr []int32, low, high int) {
	if low < high {
		p := partition(arr, low, high)
		quicksortRange(arr, low, p-1)
		quicksortRange(arr, p+1, high)
	}
}

func lcgNext(state *uint64) uint64 {
	*state = (*state)*6364136223846793005 + 1442695040888963407
	return *state
}

func main() {
	n := 100000
	var state uint64 = 88172645463325252
	data := make([]int32, n)
	for i := 0; i < n; i++ {
		data[i] = int32(lcgNext(&state) % 1000000)
	}

	quicksortRange(data, 0, n-1)

	fmt.Println(data[0], data[n-1])
}
