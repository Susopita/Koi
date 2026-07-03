package main

import "fmt"

func fib(n uint64) uint64 {
	if n < 2 {
		return n
	}
	return fib(n-1) + fib(n-2)
}

func main() {
	var n uint64 = 32
	fmt.Println(fib(n))
}
