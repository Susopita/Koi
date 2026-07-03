// Pipeline map -> filter -> reduce con closures que capturan variables
// libres (factor, threshold) por valor, midiendo el costo de closures
// nativas de Go frente al fat-pointer + struct de entorno de koi.
// Filtro simplificado a `> threshold` (sin chequeo de paridad) para que la
// comparacion con koi/lambda_map.koi sea equivalente -- koi no tiene un
// builtin `mod`.
package main

import "fmt"

func main() {
	n := 1000000
	data := make([]int64, n)
	for i := 0; i < n; i++ {
		data[i] = int64(i)
	}

	var factor int64 = 3
	doubleFn := func(x int64) int64 { return x * factor }

	var threshold int64 = 500000
	keepFn := func(x int64) bool { return x > threshold }

	var sum int64
	for _, x := range data {
		doubled := doubleFn(x)
		if keepFn(doubled) {
			sum += doubled
		}
	}

	fmt.Println(sum)
}
