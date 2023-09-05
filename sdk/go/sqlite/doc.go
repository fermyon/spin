// Package sqlite provides an interface to sqlite database stores within Spin
// components.
//
// This package is implemented as a driver that conforms to the built-in
// database/sql interface.
//
//	db := sqlite.Open("default")
//	defer db.Close()
//
//	s, err := db.Prepare("REPLACE INTO pets VALUES (4, 'Maya', ?, false);")
//	// if err != nil { ... }
//
//	_, err = s.Query("bananas")
//	// if err != nil { ... }
//
//	rows, err := db.Query("SELECT * FROM pets")
//	// if err != nil { ... }
//
//	var pets []*Pet
//	for rows.Next() {
//		var pet Pet
//		if err := rows.Scan(&pet.ID, &pet.Name, &pet.Prey, &pet.IsFinicky); err != nil {
//			...
//		}
//		pets = append(pets, &pet)
//	}
package sqlite
