BDO Huffman Decoder
====================

This project explores a few different approaches implementing a specific Huffman
encoding decoder to decompress packet contents in responses from calls to BDO's
GetWorldMarket endpoints.

## Usage
Clone the repo and run either:

- `cargo test`
- `cargo bench`

## Full Packet Processing Results

The table below highlights how data layout and algorithm choice impacted
performance. The different design decisions are detailed later.

The timings in the table include all steps to parse and decode the packet.
Times are the average 1,000,000 samples, lower is better.

| Approach     | Safety | Child |    70.5k     |    33.3k     |    22.5k     |    11.1k     |     5.5k     |     40b      | Decoding Throughput¹ |
|--------------|:------:|:-----:|:------------:|:------------:|:------------:|:------------:|:------------:|:------------:|:--------------------:|
| **BaseLine** |   ✅    |  Box  |   349.7 µs   |   162.0 µs   |   107.5 µs   |   44.81 µs   |   28.61 µs   |   715.2 ns   |      201.3 MB/s      |
| **Nested**   |   ✅    |  Box  |   298.8 µs   |   134.3 µs   |   77.56 µs   |   26.20 µs   |   17.28 µs   |   688.1 ns   |      236.9 MB/s      |
| **Nested**   |   ❌    |  Box  |   177.1 µs   |   62.94 µs   |   27.10 µs   |   13.66 µs   |   8.403 µs   |   650.6 ns   |      400.9 MB/s      |
| **Flat**     |   ✅    | Index |   301.7 µs   |   137.9 µs   |   78.61 µs   |   27.13 µs   |   18.14 µs   |   268.0 ns   |      234.3 MB/s      |
| **Flat**     |   ❓    | Const |   177.8 µs   |   64.10 µs   |   27.59 µs   |   13.58 µs   |   7.992 µs   |   255.2 ns   |      396.9 MB/s      |
| **Flat**     |   ❌    | Const |   175.1 µs   |   61.49 µs   |   26.59 µs   |   13.10 µs   |   7.992 µs   | **243.4 ns** |      403.4 MB/s      |
| **Table**    |   ✅    | Index |   60.27 µs   |   29.04 µs   |   19.73 µs   |   10.49 µs   |   6.419 µs   |   1.836 µs   |      1.204 GB/s      |
| **Table**    |   ❓    | Const |   59.77 µs   |   28.46 µs   |   19.17 µs   |   9.909 µs   |   5.747 µs   |   1.126 µs   |      1.204 GB/s      |
| **Table**    |   ❌    | Const | **56.58 µs** | **27.00 µs** | **18.27 µs** | **9.442 µs** | **5.342 µs** |   1.117 µs   |      1.271 GB/s      |

✅ Entirely safe code; no unsafe operations anywhere.  
❓ Uses only const pointer dereferences as the sole unsafe operation, otherwise safe.  
❌ Includes many unsafe practices like unchecked accesses, raw pointer manipulations, and other explicit unsafe operations.  
¹Measured 70.5k length message with decoded symbols as the unit.

Tested on a Ryzen 5700G.

## Decoding Approaches

In the world of data compression the message sizes are miniscule. This means
many of the algorithms and optimizations used in decompression libraries are not
beneficial since any overhead in the preparation of the decoding would have a
difficult time being amortized away before decoding is completed. Also, since
the symbols are dynamic and the encoding scheme is set any optimizations that
require specific encoding or embeddings are also not useful.


### Baseline:

This implementation uses a fully safe, direct traversal of a nested `TreeNode`
structure to decode the Huffman-encoded message. It initializes a `BitVec` from
the encoded message bytes and iterates through the bitstream one bit at a time.
It branches at each bit to the `left_child` or `right_child` of the current
node and resets to the root when a node with `Some(symbol)` value is reached
and appended to the decoded message `String`.

#### Key Points:

- **Safe code:** Uses `Option<Box<TreeNode>>` for child references.
- **Minimal preparation cost:** Only requires building the Huffman tree.
- **Simple and direct:** Provides a clean and readable implementation but lacks
  performance optimizations.

### TreeNode:

This implementation builds on the `Baseline` approach but uses an optimized
loop to process each encoded byte more efficiently. The decoder reads one byte
at a time in an outer loop and processes each bit in an inner loop. The
decoded message is written to a pre-allocated Vec<u8>.

#### Key Improvements:

- **Inner loop optimization:** The loop over the bits of each byte is fully
  unrolled by the compiler, using a single byte pointer, eliminating conditional
  checks for better performance.
- **String elimination:** Replaces the `String` with a `Vec<u8>` which is
  allocated up-front, assigned to and then converted to a `String` using
  `from_utf8`.

#### Safety:

- **Iteration**: The safe version uses an iterator while the unsafe version uses
  a simple `loop` and unchecked gets for the bytes.
- **Reduced branch misprediction:** Double-pointer indirection minimizes
  branching during tree traversal, improving CPU pipeline efficiency.
- **Decoded message**: The safe version assigns to each indexed element of the
  decoded message while the unsafe version assigns symbols via a ptr and offset.


### FlatNode:

This approach builds on the TreeNode approach but uses a flat representation of
the tree instead of a nested representation. Using this flat layout allows
either indices or pointers to be used for child node linking.

An observation of the TreeNode approach for small messages is that building the
heap (tree) is ~85% of the total processing time. Using a flat representation
allows for a simpler Node and reduces the heap (tree) building time by 75% so
that it accounts for less than 50% of the time on the small messages.

#### Key Improvements:

- **Array-based traversal:** reduces upfront costs of tree building.

#### Safety:

- **Iteration**: The safe version uses an iterator while the unsafe version uses
  a simple `loop` and dereferencing of const ptrs.


### Table:

This approach uses a multi-symbol lookup table built upfront by decoding all
8-bit paths through the tree.

An observation from analysis of the FlatNode approach is that the upfront cost
of any prep work for faster decoding has to be amortized quickly when dealing
with only 75k or less encoded bytes. After several failed experiments with a
variety of table based, recursive trees, multi-symbol processing methods,
SIMD, parallel, finite state machines and others I was ready to say it was just
too small to amortize the setup costs.

Using the FlatNode decoder to decode integers `0..=255` and storing the symbols
traversed for each along with the symbol count and number of bits used each in
their own flat array provides excellent data characteristics for the compiler
and the cpu.

#### Key Improvements:

- **[Bitter](https://github.com/nickbabcock/bitter):** for
  [blazing fast bit reading](https://github.com/nickbabcock/bitter#comparison-to-other-libraries).
- **Multi-symbol:** flat lookup table for each 8 step path from `0..=255`
  generated using the tree provided by `FlatNode`.
- **Manual loop unrolling:** for both bulk ('outer') and per symbol ('inner')
  loops.

#### Safety:

- **Iteration:** on both the fully safe and safe except const pointer deref
  versions use the same safe decoding process, the difference is in the tree
  building. The unsafe version uses mut ptr writes and unchecked reads.
- **Bit buffering:** is fully checked in the safe version and unchecked for all
  bits except the tail in the unsafe version.


## BDO's Huffman


### Packet Structure
The first thing to know is the structure of the incoming packet contents which
is easily described using [Kaitai Struct](https://kaitai.io/):

```yaml
meta:
  id: huffman_packet
  endian: le

seq:
  - id: len_content
    type: u8
  - id: len_symbol_table
    type: u4
  - id: symbol_table
    size: 8 * len_symbol_table
  - id: len_bitstream
    type: u4
  - id: len_encoded_data
    type: u4
  - id: len_decoded_data
    type: u4
  - id: message
    size: len_encoded_data

instances:
  symbol_entries:
    io: _root._io
    pos: 12
    type: symbol_entry
    repeat: expr
    repeat-expr: _root.len_symbol_table
  decoded_message:
    pos: 24 + ( 8 * _root.len_symbol_table)
    size: _root.len_encoded_data
    process: huffman_decoder(_root.len_symbol_table, _root.symbol_table, _root.len_bitstream)

types:
  symbol_entry:
    seq:
      - id: frequency
        type: u4
      - id: symbol
        type: strz
        size: 4
        encoding: UTF8
```

The symbol table can consist of `-`, `0-9` and `|`.

Once decoded '|' and '-' denote record and field delimiters respectively, they
will always be present in the table. There are four fields in each record:
'item', 'count', 'price' and 'cumulative count'.

This repo is only concerned with decoding so parsing the decoded message is out
of scope but if someone was parsing the decoded messages (perhaps to feed into a
database) then incorporating the record structure and emitting an update message
into the decoder should be very efficient vs post processing.

#### A note on the symbol table and prefix lengths.

When parsing the packet ensure that whatever container is used to store the
symbol and frequency has stable order! The input order is crucial to the proper
building of the Huffman tree.

The minimum prefix length is 1, the maximum observed prefix is 7.

### The Heap

Since the incoming messages are already encoded we must use a Min-Heap that
orders the symbol nodes the same way to decode them. BDO's encoding
matches the ordering of the the classic Min-BinaryHeap. Many modern heap
implementations are optimized and do not provide the ordering required and if
they do provide the right ordering they may be sub-optimal for this use case.

What you want is a simple collection and simple _sift-up_ and _sift-down_
procedures implemented directly as _push_ and _pop_ functions respectively.

A simple Min-Heap example implementation in python where `heap` is a simple list
and `node` is a simple object containing a comparison function using 'frequency'
would look something like this.

```python
def push(heap, node):
  heap.append(node)
  child = len(heap) - 1
  while child > 0:
    parent = (child - 1) >> 1
    if heap[child] < heap[parent]:
      heap[parent], heap[child] = heap[child], heap[parent]
    child = parent

def pop(heap):
  node = heap.pop()
  if heap:
    node, heap[0] = heap[0], node
    parent, child, end = 0, 1, len(heap)
    while child < end:
      i = child if heap[child] < heap[parent] else parent
      child += 1
      i = child if (child < end and heap[child] < heap[i]) else i
      if parent == i:
        break
      heap[parent], heap[i] = heap[i], heap[parent]
      parent, child = i, 2 * i + 1
  return node
```

*Whatever min-heap you use the only truly important thing is that the popping
order matches and is deterministic.*

## Message Sizes

The response of a GetWorldMarket request is for a particular main and sub
category pair. The sizes of these categories varies dramatically with the
largest being >70kb decompressed and the smallest only 40b.

| Group        | Main | Sub | Decoded Size |
|:-------------|------|-----|--------------|
| large        | 55   | 4   | 70.5k        |
| large_medium | 55   | 3   | 33.3k        |
| medium       | 55   | 2   | 22.5k        |
| medium_small | 55   | 1   | 11.1k        |
| small        | 25   | 2   | 5.5k         |
| small_min    | 75   | 6   | 40b          |

While the message size is not exactly tied to the number of items in a category
it is directly tied to it.

 _(See
[bdoMarket Master Items Table](
https://docs.google.com/spreadsheets/d/1LFri67Eb2nW8VmoG7FGNXIhGAexqGxdZnNQqvzCm-dw
) for categorized data to analyze.)_
