package core

import (
	"crypto/sha256"
	"fmt"
	"io/ioutil"
	"log"
	"math/big"
	"math/rand"
	"os"
	"sort"
	"strconv"
	"testing"
	"time"

	fuzz "github.com/AdaLogics/go-fuzz-headers"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/consensus/ethash"
	"github.com/ethereum/go-ethereum/core/rawdb"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/ethdb"
	"github.com/ethereum/go-ethereum/params"
)

type Triple struct {
	x, y, z int
}

var reorg_types = make(map[Triple]bool)
var fork_types = make(map[[32]byte]bool)
var previous_mode_byte byte = 0xFF
var previous_block *types.Block

var is_invalid = false

type TreeBlock struct {
	ethBlock *types.Block
	depth    int
	Parent   *TreeBlock
	Children []*TreeBlock
}

func isomorphic_code(root *TreeBlock) string {
	if root == nil {
		return ""
	}

	childCodes := []string{}
	for _, child := range root.Children {
		childCodes = append(childCodes, isomorphic_code(child))
	}

	sort.Strings(childCodes)

	// code := "("
	n := 1
	code := ""
	for _, childCode := range childCodes {
		code += childCode
		n += int(childCode[0])
	}
	code = strconv.Itoa(n) + code

	return code
}

// func bytesToString(data []byte) string {
// 	result := ""
// 	for _, b := range data {
// 		if b >= 0 && b <= 127 { // ASCII printable characters
// 			result += string(b)
// 		} else {
// 			result += fmt.Sprintf("%q", b)
// 		}
// 	}
// 	return result
// }

func calculateSHA256(str string) string {
	hash := sha256.Sum256([]byte(str))
	return fmt.Sprintf("%x", hash[:8])
}

func findNonLeafNodes(root *TreeBlock, result *[]*TreeBlock, visited map[*TreeBlock]bool) {
	if _, ok := visited[root]; !ok && len(root.Children) > 0 {
		*result = append(*result, root)
		visited[root] = true
	}
	for _, child := range root.Children {
		findNonLeafNodes(child, result, visited)
	}
}

func traverseTreeBlock(root *TreeBlock, result *[]*TreeBlock, visited map[*TreeBlock]bool) {
	if _, ok := visited[root]; !ok {
		*result = append(*result, root)
		visited[root] = true
	}
	for _, child := range root.Children {
		traverseTreeBlock(child, result, visited)
	}
}

func sortTreeBlocksByDepth(blocks []*TreeBlock) {
	sort.Slice(blocks, func(i, j int) bool {
		return blocks[i].depth > blocks[j].depth
	})
}

func find_depth(root *TreeBlock) *TreeBlock {
	var blocks []*TreeBlock
	visited := make(map[*TreeBlock]bool)
	traverseTreeBlock(root, &blocks, visited)
	sortTreeBlocksByDepth(blocks)

	return blocks[0]
}

func find_balance(root *TreeBlock) *TreeBlock {
	var blocks []*TreeBlock
	visited := make(map[*TreeBlock]bool)
	traverseTreeBlock(root, &blocks, visited)
	sortTreeBlocksByDepth(blocks)

	maxDepth := blocks[0].depth
	for _, block := range blocks {
		if block.depth < maxDepth {
			return block
		}
	}

	return blocks[0]
}

func find_breadth(root *TreeBlock) *TreeBlock {
	var nonLeafNodes []*TreeBlock
	visited := make(map[*TreeBlock]bool)
	findNonLeafNodes(root, &nonLeafNodes, visited)

	rand.Seed(time.Now().UnixNano())
	if len(nonLeafNodes) == 0 {
		return find_depth(root)
	}
	return nonLeafNodes[rand.Intn(len(nonLeafNodes))]
}

func get_next_block(data []byte, tree_root *TreeBlock, fuzzConsumer *fuzz.ConsumeFuzzer, blockchain *BlockChain, db ethdb.Database, logger *log.Logger) (*types.Block, error) {

	block := &types.Block{}

	parent := tree_root

	BYTE_MAX := 0xFF
	probDepth := byte(0.3 * float64(BYTE_MAX))
	probBreadth := byte(0.3 * float64(BYTE_MAX))
	probBalance := byte((BYTE_MAX)) - probBreadth - probDepth

	r, _ := fuzzConsumer.GetByte()

	logger.Printf("Random byte: %d, probDepth: %d, probBreadth: %d\n", r, probDepth, probBreadth)
	if r < probBalance {
		parent = find_balance(tree_root)
	} else if r < probBreadth+probBalance {
		parent = find_breadth(tree_root)
	} else {
		parent = find_depth(tree_root)
	}

	// New TreeBlock
	// block = makeBlockChain(parent.ethBlock, 1, ethash.NewFaker(), db, method)[0]
	// makeBlockChain creates a deterministic chain of blocks rooted at parent.
	// func makeBlockChain(parent *types.Block, n int, engine consensus.Engine, db ethdb.Database, seed int) []*types.Block {
	var (
		key, _  = crypto.HexToECDSA("b71c71a67e1177ad4e901695e1b4b9ee17ae16c6668d313eac2f96dbcda3f291")
		address = crypto.PubkeyToAddress(key.PublicKey)
		gspec   = &Genesis{
			Config: params.TestChainConfig,
		}
		signer = types.NewEIP155Signer(gspec.Config.ChainID)
	)

	if previous_mode_byte == r {
		block = previous_block
	} else {
		blocks, _ := GenerateChain(params.TestChainConfig, parent.ethBlock, ethash.NewFaker(), db, 1, func(i int, b *BlockGen) {
			// b.SetCoinbase(common.Address{0: byte(method), 19: byte(i)})
			b.SetCoinbase(address)
			// Generate < 20 transactions
			// nTx := method % 20
			ntx, _ := fuzzConsumer.GetByte()
			nTx := int(ntx % 20)

			namount, _ := fuzzConsumer.GetByte()
			nAmount := int(namount % 100)

			nfuzztx, _ := fuzzConsumer.GetByte()
			nFuzztx := int(nfuzztx % 0xff)

			seed, _ := fuzzConsumer.GetInt()
			for j := 0; j < nTx; j++ {
				tx, err := types.SignTx(types.NewTransaction(b.TxNonce(address), address, big.NewInt(int64(nAmount)), params.TxGas, nil, nil), signer, key)
				if err != nil {
					panic(err)
				}
				b.AddTx(tx)
			}
			if (nFuzztx) == 0 {
				tx, err := types.SignTx(types.NewFuzzTransaction(b.TxNonce(address), address, big.NewInt(100), params.TxGas, nil, nil, seed), signer, key)
				if err != nil {
					panic(err)
				}
				b.AddTx(tx)
			}

			nfuture, _ := fuzzConsumer.GetByte()
			nFuture := int(nfuture % 0xff)
			if nFuture == 0 {
				// Modify current block's timestamp
				timestamp, _ := fuzzConsumer.GetInt()
				b.OffsetTime(int64(timestamp))
			}
		})
		block = blocks[0]

		new_treeblock := &TreeBlock{ethBlock: block, depth: parent.depth + 1, Parent: parent}
		parent.Children = append(parent.Children, new_treeblock)
	}

	previous_block = block
	previous_mode_byte = r
	return block, nil
}

func TestForky(t *testing.T) {
	files, err := os.ReadDir("./corpus/FuzzForky/")
	if err != nil {
		log.Fatal(err)
	}

	// ----------------------------------------------------------
	// Initialize empty blockchain
	var (
		gendb   = rawdb.NewMemoryDatabase()
		key, _  = crypto.HexToECDSA("b71c71a67e1177ad4e901695e1b4b9ee17ae16c6668d313eac2f96dbcda3f291")
		address = crypto.PubkeyToAddress(key.PublicKey)
		funds   = big.NewInt(1000000000)
		// _ = new(Genesis).MustCommit(db)
		gspec = &Genesis{
			Config: params.TestChainConfig,
			Alloc:  GenesisAlloc{address: {Balance: funds}},
		}
		genesis = gspec.MustCommit(gendb)
		signer  = types.NewEIP155Signer(gspec.Config.ChainID)
	)
	height := uint64(50)
	// blocks, _ := GenerateChain(gspec.Config, genesis, ethash.NewFaker(), gendb, int(height), nil)
	blocks, _ := GenerateChain(gspec.Config, genesis, ethash.NewFaker(), gendb, int(height), func(i int, block *BlockGen) {
		tx, err := types.SignTx(types.NewTransaction(block.TxNonce(address), common.Address{0x00}, big.NewInt(1000), params.TxGas, nil, nil), signer, key)
		if err != nil {
			panic(err)
		}
		block.AddTx(tx)
	})

	// Print blocks' hashes into list
	for _, block := range blocks {
		fmt.Println("Block hash: ", block.Hash())
	}

	// makeDb creates a db instance for testing.
	makeDb := func() (ethdb.Database, func()) {
		dir, _ := ioutil.TempDir("", "")
		// if err != nil {
		// 	t.Fatalf("failed to create temp freezer dir: %v", err)
		// }
		defer os.Remove(dir)
		db, _ := rawdb.NewDatabaseWithFreezer(rawdb.NewMemoryDatabase(), dir, "")
		// if err != nil {
		// 	t.Fatalf("failed to create temp freezer db: %v", err)
		// }
		gspec.MustCommit(db)
		return db, func() { os.RemoveAll(dir) }
	}
	db, delfn := makeDb()
	defer delfn()

	archiveCaching := *defaultCacheConfig
	archiveCaching.TrieDirtyDisabled = true

	// Initialize a fresh chain with only a genesis block to blockheight = height (50)
	blockchain, _ := NewBlockChain(db, &archiveCaching, gspec.Config, ethash.NewFaker(), vm.Config{}, nil, nil)
	blockchain.InsertChain(blocks)
	defer blockchain.Reset()
	defer blockchain.Stop()
	// ----------------------------------------------------------

	for _, file := range files {
		if file.IsDir() {
			continue
		}

		// Read file into bytes
		data, err := os.ReadFile("./corpus/FuzzForky/" + file.Name())
		// log.Println("Executing testForky with file: ", file.Name())
		if err != nil {
			log.Fatal(err)
		}

		data_to_test, _ := unmarshalCorpusFile(data)

		testForky(t, data_to_test[0].([]byte), blockchain, db)
		if err != nil {
			log.Fatal(err)
		}
		blockchain.SetHead(uint64(height))
	}
}

func testForky(t *testing.T, data []byte, blockchain *BlockChain, db ethdb.Database) {
	file, err := os.OpenFile("test-TestForky.log", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		log.Fatal(err)
	}
	defer file.Close()
	logger := log.New(file, "", log.LstdFlags)
	logger.Printf("---- New test ----\n")

	debug_file, err := os.OpenFile("debug-TestForky.log", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		log.Fatal(err)
	}
	defer debug_file.Close()
	debug_logger := log.New(debug_file, "", log.LstdFlags)

	debug_logger.Println("data: ", data)
	debug_logger.Println("marshal: ", string(marshalCorpusFile(data)))

	fuzzConsumer := fuzz.NewConsumer(data)
	// Number of blocks: 1 byte
	num_blocks, _ := fuzzConsumer.GetInt()
	// logger.Println("Number of blocks: ", num_blocks, num_blocks%10)
	// logger.Println("Current head: ", blockchain.CurrentBlock().Hash())
	num_blocks = num_blocks%64 + 1

	// Tree view
	tree_root := &TreeBlock{ethBlock: blockchain.CurrentBlock(), depth: int(blockchain.CurrentBlock().NumberU64())}

	nth_reorg := 0
	is_invalid = false
	for i := 0; i < num_blocks; i++ {
		// Create new block to process
		// method, err := fuzzConsumer.GetInt()
		if err != nil {
			logger.Printf("Number of blocks: %d, but not enough bytes", num_blocks)
			t.SkipNow()
		}

		block, err := get_next_block(data, tree_root, fuzzConsumer, blockchain, db, logger)
		if err != nil {
			logger.Println("Error creating a block: ", err)
			t.SkipNow()
		}

		// tip_before := blockchain.CurrentBlock().NumberU64()

		// logger.Println("New block parent: ", block.ParentHash())
		// Process the block
		// logger.Println("Processing the block")
		n_replacing, n_replaced, err_int, err := blockchain.InsertChain2([]*types.Block{block})
		// logger.Println("Block processed")
		if n_replacing != -1 || n_replaced != -1 {
			// logger.Println("nth_reorg, n_replacing, n_replaced: ", nth_reorg, n_replacing, n_replaced)
			nth_reorg += 1
		} else {
			// logger.Println("No_reorg")
		}
		if err != nil {
			logger.Println("Error during processing a block: ", err_int, err.Error())
			is_invalid = true
		}
	}
}

func FuzzForky(f *testing.F) {
	// Generate Seed
	// Initialize empty blockchain
	// var (
	// 	dbs      = rawdb.NewMemoryDatabase()
	// 	_ = new(Genesis).MustCommit(dbs)
	// )

	// // Initialize a fresh chain with only a genesis block
	// bc, _ := NewBlockChain(dbs, nil, params.AllEthashProtocolChanges, ethash.NewFaker(), vm.Config{}, nil, nil)

	// bks := makeBlockChain(bc.CurrentBlock(), 10, ethash.NewFaker(), dbs, 2)

	// var blockBytes [][]byte
	// blockBytes = append(blockBytes, []byte{0x10}) // Convert 0x10 to a byte slice

	// for _, block := range bks {
	// 	var buf = bytes.NewBuffer(make([]byte, 0, 32000))
	// 	if err := rlp.Encode(buf, block); err != nil { fmt.Print(err); return }
	// 	// fmt.Println("Block hash: ", block.Hash(), "length: ", len(buf.Bytes()))
	// 	blockBytes = append(blockBytes, []byte{0xff})
	// 	blockBytes = append(blockBytes, []byte{byte(len(buf.Bytes()))}) // Convert the length to a byte value
	// 	blockBytes = append(blockBytes, buf.Bytes())

	// 	// test
	// 	decoded_block := new(types.Block)
	// 	if err := rlp.Decode(buf, decoded_block); err != nil { fmt.Print(err); return }
	// 	// fmt.Println(block.Header())
	// 	// fmt.Println(decoded_block.Header())

	// 	if block.Header().Hash() != decoded_block.Header().Hash() { fmt.Println("Hashes don't match"); return }
	// }

	// dataBytes := bytes.Join(blockBytes, []byte(""))
	// f.Add(dataBytes)
	// // fmt.Println("Seed corpus added, length: ", len(dataBytes))
	// bc.Stop()

	// ----------------------------------------------------------
	// Initialize empty blockchain
	var (
		gendb   = rawdb.NewMemoryDatabase()
		key, _  = crypto.HexToECDSA("b71c71a67e1177ad4e901695e1b4b9ee17ae16c6668d313eac2f96dbcda3f291")
		address = crypto.PubkeyToAddress(key.PublicKey)
		funds   = big.NewInt(1000000000)
		// _ = new(Genesis).MustCommit(db)
		gspec = &Genesis{
			Config: params.TestChainConfig,
			Alloc:  GenesisAlloc{address: {Balance: funds}},
		}
		genesis = gspec.MustCommit(gendb)
		signer  = types.NewEIP155Signer(gspec.Config.ChainID)
	)
	height := uint64(50)
	// blocks, _ := GenerateChain(gspec.Config, genesis, ethash.NewFaker(), gendb, int(height), nil)
	blocks, _ := GenerateChain(gspec.Config, genesis, ethash.NewFaker(), gendb, int(height), func(i int, block *BlockGen) {
		tx, err := types.SignTx(types.NewTransaction(block.TxNonce(address), common.Address{0x00}, big.NewInt(1000), params.TxGas, nil, nil), signer, key)
		if err != nil {
			panic(err)
		}
		block.AddTx(tx)
	})

	// Print blocks' hashes into list
	for _, block := range blocks {
		fmt.Println("Block hash: ", block.Hash())
	}

	// makeDb creates a db instance for testing.
	makeDb := func() (ethdb.Database, func()) {
		dir, _ := ioutil.TempDir("", "")
		// if err != nil {
		// 	t.Fatalf("failed to create temp freezer dir: %v", err)
		// }
		defer os.Remove(dir)
		db, _ := rawdb.NewDatabaseWithFreezer(rawdb.NewMemoryDatabase(), dir, "")
		// if err != nil {
		// 	t.Fatalf("failed to create temp freezer db: %v", err)
		// }
		gspec.MustCommit(db)
		return db, func() { os.RemoveAll(dir) }
	}
	db, delfn := makeDb()
	defer delfn()

	archiveCaching := *defaultCacheConfig
	archiveCaching.TrieDirtyDisabled = true

	// Initialize a fresh chain with only a genesis block to blockheight = height (50)
	blockchain, _ := NewBlockChain(db, &archiveCaching, gspec.Config, ethash.NewFaker(), vm.Config{}, nil, nil)
	blockchain.InsertChain(blocks)
	defer blockchain.Reset()
	defer blockchain.Stop()
	// ----------------------------------------------------------

	file, err := os.OpenFile("test.log", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		log.Fatalf("Error opening file: %v", err)
	}
	defer file.Close()

	debug_file, err := os.OpenFile("debug.log", os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		log.Fatalf("Error opening file: %v", err)
	}
	defer debug_file.Close()
	debug_logger := log.New(debug_file, "", log.LstdFlags)

	fmt.Println("Init Done")

	f.Fuzz(
		func(t *testing.T, data []byte) {
			// Rewinds th blockchain to a new head
			defer blockchain.SetHead(uint64(height))

			logger := log.New(file, "", log.LstdFlags)
			// logger.Printf("New test\n")

			// debug_logger.Println("data: ", data)
			// debug_logger.Println("marshal: ", string(marshalCorpusFile(data)))
			corpusString := string(marshalCorpusFile(data))

			// unmarshal, _ := unmarshalCorpusFile([]byte(corpusString))
			// debug_logger.Println("unmarshal: ", unmarshal[0])
			// debug_logger.Println("Data: ", hexData)
			// str := "go test fuzz v1\n[]byte(\"" + hexData + "\")\n"
			filename := calculateSHA256(corpusString)

			fuzzConsumer := fuzz.NewConsumer(data)
			// Number of blocks: 1 byte
			num_blocks, _ := fuzzConsumer.GetInt()
			// logger.Println("Number of blocks: ", num_blocks, num_blocks % 10)
			num_blocks = num_blocks%32 + 1

			// Tree view
			tree_root := &TreeBlock{ethBlock: blockchain.CurrentBlock(), depth: int(blockchain.CurrentBlock().NumberU64())}

			nth_reorg := 0
			found_new_reorg_type := false
			is_invalid = false
			for i := 0; i < num_blocks; i++ {
				// Create new block to process
				// method, err := fuzzConsumer.GetInt()
				if err != nil {
					logger.Printf("Number of blocks: %d, but not enough bytes", num_blocks)
					t.SkipNow()
				}

				block, err := get_next_block(data, tree_root, fuzzConsumer, blockchain, db, logger)
				if err != nil {
					logger.Println("Error creating a block: ", err)
					t.SkipNow()
				}

				// tip_before := blockchain.CurrentBlock().NumberU64()

				// Process the block
				// logger.Println("Processing the block")
				n_replacing, n_replaced, err_int, err := blockchain.InsertChain2([]*types.Block{block})
				// logger.Println("Block processed")
				if n_replacing != -1 || n_replaced != -1 {
					debug_logger.Println("nth_reorg, n_replacing, n_replaced: ", nth_reorg, n_replacing, n_replaced)
					nth_reorg += 1
				} else {
					// logger.Println("No_reorg")
				}
				if err != nil {
					logger.Println("Error during InsertChain: ", err_int, err.Error())
					is_invalid = true
				}

				new_state := Triple{int(nth_reorg), int(n_replacing), int(n_replaced)}
				if _, ok := reorg_types[new_state]; !ok {
					reorg_types[new_state] = true
					found_new_reorg_type = true
				}

				// tip_after := blockchain.CurrentBlock().NumberU64()
				// logger.Printf("Block hash: %x, parent hash: %x", block.Hash(), block.ParentHash())
				// logger.Println("** Tip before: ", tip_before, "Tip after: ", tip_after)
			}
			// logger.Println("----------------------------------------------------------")

			// test
			// if found_new_reorg_type  {
			// 	file, err := os.Create("./corpus/FuzzForky/" + filename)
			// 	if err != nil {
			// 		log.Fatal(err)
			// 	}

			// 	// Write the string to the file
			// 	_, err = file.WriteString(str)
			// 	if err != nil {
			// 		log.Fatal(err)
			// 	}

			// 	// Close the file
			// 	err = file.Close()
			// 	if err != nil {
			// 		log.Fatal(err)
			// 	}
			// }

			// found_new_fork_type := true
			code := isomorphic_code(tree_root)
			// Add code into fork_type map
			hash := sha256.Sum256([]byte(code))
			if _, ok := fork_types[hash]; ok {
				fork_types[hash] = false
			}

			// if !is_invalid && (found_new_reorg_type || found_new_fork_type) {
			if !is_invalid && (found_new_reorg_type) {
				file, err := os.Create("./corpus/FuzzForky/" + filename)
				// logger.Println("Fork type found: ", code)
				if err != nil {
					log.Fatal(err)
				}

				// Write the string to the file
				_, err = file.WriteString(corpusString)
				if err != nil {
					log.Fatal(err)
				}

				// Close the file
				err = file.Close()
				if err != nil {
					log.Fatal(err)
				}
			}

		})
}
