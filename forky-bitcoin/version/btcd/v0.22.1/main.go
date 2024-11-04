package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/btcsuite/btcd/blockchain"
	"github.com/btcsuite/btcd/chaincfg"
	"github.com/btcsuite/btcd/chaincfg/chainhash"
	"github.com/btcsuite/btcd/database"
	"github.com/btcsuite/btcutil"

	_ "github.com/btcsuite/btcd/database/ffldb"
)

type Pair struct {
	tx_hash    string
	block_hash *chainhash.Hash
}

var separator = []byte{0xFA, 0xBF, 0xB5, 0xDA}

var utxoSetBucketName = []byte("utxosetv2")

const mocktime = 1637053432

func Enqueue(queue []Pair, tx_hash string, block_hash *chainhash.Hash, limit int) []Pair {
	pair := Pair{tx_hash: tx_hash, block_hash: block_hash}
	queue = append(queue, pair)
	if len(queue) > limit {
		queue = queue[1:]
	}
	return queue
}

func check(e error) {
	if e != nil {
		panic(e)
	}
}

func main() {

	// Database filepath
	dbPath := filepath.Join(os.TempDir(), "btcd")

	// Create a database
	os.RemoveAll(dbPath)
	db, err := database.Create("ffldb", dbPath, chaincfg.RegressionNetParams.Net)
	check(err)

	timesource := blockchain.NewMedianTime()
	timesource.SetMockTime(mocktime)

	// Create a blockchain
	chain, err := blockchain.New(&blockchain.Config{
		DB:          db,
		ChainParams: &chaincfg.RegressionNetParams,
		TimeSource:  timesource,
	})
	check(err)

	//blockchain.KeyToHash = map[string]string{}

	// Import blocks
	dat, err := os.ReadFile("./import/import.dat")
	check(err)
	var imported_blocks_raw = bytes.Split(dat, separator)

	// Remove first block if empty
	if len(imported_blocks_raw[0]) == 0 {
		imported_blocks_raw = imported_blocks_raw[1:]
	}

	// Parse all improted blocks
	var imported_txos []*chainhash.Hash
	for _, block_raw := range imported_blocks_raw[1:] {
		block, err := btcutil.NewBlockFromBytes(block_raw[4:])
		check(err)
		chain.ProcessBlock(block, blockchain.BFNone)
		for _, txo := range block.Transactions() {
			imported_txos = append(imported_txos, txo.Hash())
		}
	}

	// Read all test cases
	test_cases, err := os.ReadDir("./test_cases")
	check(err)

	//Process all test cases
	for _, test_case := range test_cases {
		// Print test case name
		fmt.Println(test_case.Name())

		// Import all test blocks
		dat, err := os.ReadFile(filepath.Join("./test_cases", test_case.Name()))
		check(err)
		test_blocks := bytes.Split(dat, separator)

		// Remove first block if empty
		if len(test_blocks[0]) == 0 {
			test_blocks = test_blocks[1:]
		}

		// Try adding each block to the chain
		result := map[string]interface{}{}
		var txos []*chainhash.Hash

		var blocks []*btcutil.Block

		coinbase := []Pair{}

		var utxos []string
		var utxos_set = make(map[string]bool)

		for i, block_raw := range test_blocks {

			block, err := btcutil.NewBlockFromBytes(block_raw[9:])

			if err != nil {
				result["Block "+strconv.Itoa(i)] = map[string]interface{}{"accept": false, "reason": err.Error()}
				continue
			}

			accepted, _, err := chain.ProcessBlock(block, blockchain.BFNone)

			var reason string = "Valid"

			if err != nil {
				reason = err.Error()
			} else {
				for _, txo := range block.Transactions() {
					txos = append(txos, txo.Hash())
				}
				blocks = append(blocks, block)
				coinbase = Enqueue(coinbase, block.Transactions()[0].Hash().String(), block.Hash(), 100)
			}

			result["Block "+padStringWithZeros(strconv.Itoa(i), 2)] = map[string]interface{}{"accept": accepted, "reason": reason}

			for _, txo := range coinbase {
				if chain.MainChainHasBlock(txo.block_hash) {
					if utxos_set[txo.tx_hash] == false {
						utxos = append(utxos, txo.tx_hash)
						utxos_set[txo.tx_hash] = true
					}
				}
			}
		}

		result["HashTip"] = chain.BestSnapshot().Hash.String()

		// Check whether txos are utxos
		for _, txo := range imported_txos {
			db.View(func(tx database.Tx) error {
				_, err := blockchain.DBFetchUtxoEntryByHash(tx, txo)
				if err == nil && utxos_set[txo.String()] == false {
					utxos = append(utxos, txo.String())
					utxos_set[txo.String()] = true
				}
				return nil
			})
		}

		for _, txo := range txos {
			db.View(func(tx database.Tx) error {
				entry, err := blockchain.DBFetchUtxoEntryByHash(tx, txo)
				if err == nil && entry != nil && utxos_set[txo.String()] == false {
					utxos = append(utxos, txo.String())
					utxos_set[txo.String()] = true
				}
				return nil
			})
		}

		sort.Strings(utxos)

		result["UTXO"] = utxos

		// Parse to JSON and save
		jsonData, err := json.MarshalIndent(result, "", "    ")
		check(err)
		err = os.WriteFile(filepath.Join("./output/", test_case.Name()), jsonData, 0644)
		check(err)

		// Reset chain
		chain.PurgeOrphans()
		for chain.BestSnapshot().Height >= int32(len(imported_blocks_raw)) {
			chain.RemoveLastMainBlock()
		}
		chain.RemoveLostBlocks(blocks)
	}

	os.RemoveAll(dbPath)
}

func padStringWithZeros(inputString string, desiredLength int) string {
	if len(inputString) >= desiredLength {
		return inputString // No need to pad if the string is already as long as desired.
	}

	zerosToPad := desiredLength - len(inputString)
	paddedString := strings.Repeat("0", zerosToPad)

	return paddedString + inputString
}
