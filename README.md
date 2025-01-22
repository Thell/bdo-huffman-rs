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
performance.  
Times are the average 1,000,000 samples, lower is better.

| Approach          | Safety | Child |     141k     |    70.5k     |    33.3k     |    22.5k     |    11.1k     |     5.5k     |     40b      |
|-------------------|:------:|:-----:|:------------:|:------------:|:------------:|:------------:|:------------:|:------------:|:------------:|
| **Original**      |   ✅    |  Box  |      -       |   398.4 µs   |   177.2 µs   |   113.0 µs   |   53.59 µs   |   38.98 µs   |   1.375 µs   |
| **BaseLine**      |   ✅    |  Box  |      -       |   332.6 µs   |   156.6 µs   |   102.3 µs   |   44.32 µs   |   22.98 µs   |   507.1 ns   |
|                   |        |       |              |              |              |              |              |              |              |
| **Nested**        |   ✅    |  Box  |      -       |   300.3 µs   |   134.1 µs   |   76.92 µs   |   26.74 µs   |   17.40 µs   |   528.3 ns   |
| **Flat**          |   ✅    | Index |      -       |   296.7 µs   |   133.5 µs   |   76.17 µs   |   25.41 µs   |   16.78 µs   | **192.8** ns |
| **S-Table**       |   ✅    | Index |      -       |   142.5 µs   |   64.80 µs   |   43.38 µs   |   21.53 µs   |   10.57 µs   |   277.7 ns   |
| **M-Table**       |   ✅    | Index |   114.0 µs   |   57.90 µs   | **27.85** µs | **19.13** µs | **10.09** µs | **6.643** µs |   1.781 µs   |
| **FSM**           |   ✅    | Index |   112.7 µs   |   61.35 µs   |   34.67 µs   |   26.30 µs   |   18.64 µs   |   15.73 µs   |   10.87 µs   |
| **2 Channel FSM** |   ✅    | Index | **93.80** µs | **52.45** µs |   30.52 µs   |   23.80 µs   |   17.77 µs   |   14.69 µs   |   11.23 µs   |
|                   |        |       |              |              |              |              |              |              |              |
| **Nested**        |   ❌    |  Box  |      -       |   175.5 µs   |   62.00 µs   |   26.71 µs   |   13.41 µs   |   8.109 µs   |   465.8 ns   |
| **Flat**          |   ❌    | Const |      -       |   176.2 µs   |   60.25 µs   |   26.44 µs   |   13.11 µs   |   7.755 µs   | **145.0** ns |
| **S-Table**       |   ❌    | Const |      -       |   128.7 µs   |   57.97 µs   |   38.61 µs   |   19.20 µs   |   9.372 µs   |   258.3 µs   |
| **M-Table**       |   ❌    | Const |   102.9 µs   |   51.98 µs   |   24.66 µs   | **16.83** µs | **8.573** µs | **5.298** µs |   937.1 ns   |
| **FSM**           |   ❌    | Index |   97.52 µs   |   53.76 µs   |   30.56 µs   |   23.55 µs   |   16.84 µs   |   14.84 µs   |   10.15 µs   |
| **2 Channel FSM** |   ❌    | Index | **63.25** µs | **36.68** µs | **22.52** µs |   18.14 µs   |   14.07 µs   |   12.94 µs   |   10.10 µs   |

✅ Entirely safe code; no unsafe operations anywhere.  
❌ Includes unsafe practices like unchecked accesses, raw pointer manipulations, and other explicit unsafe operations.  
'**Original**' and '**Baseline**' have the same decoding logic, timing changes
come from the packet parsing and tree building.  
'**S-Table**'' and '**M-Table**'' use single-symbol and multi-symbol lookup tables.  

The timings in the table include all steps to parse and decode the packet.  
All aproaches use the same parser, except the original, which takes less than 3ns for all packets.  
Tested on a Ryzen 5700G.

## Decoding Approaches

In the world of data compression the message sizes are miniscule. This means
many of the algorithms and optimizations used in decompression libraries are not
beneficial since any overhead in the preparation of the decoding would have a
difficult time being amortized away before decoding is completed. Also, since
the symbols are dynamic and the encoding scheme is set any optimizations that
require specific encoding or embeddings are also not useful.


### Baseline:

This implementation uses a fully safe, direct traversal of a nested tree
structure to decode the Huffman-encoded message. It initializes a `BitVec` from
the encoded message bytes and iterates through the bitstream one bit at a time.
It branches at each bit to the `left_child` or `right_child` of the current
node and resets to the root when a node with `Some(symbol)` value is reached
and appended to the decoded message `String`.

#### Key Points:

- **Safe code:** Uses `Option<Box<Node>>` for child references.
- **Minimal preparation cost:** Only requires building the Huffman tree.
- **Simple and direct:** Provides a clean and readable implementation but lacks
  performance optimizations.

### Nested:

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


### Flat:

This approach builds on the nested approach but uses a flat representation of
the tree instead of a nested representation. Using this flat layout allows
either indices or pointers to be used for child node linking.

An observation of the nested approach for small messages is that building the
heap (tree) is ~85% of the total processing time. Using a flat representation
allows for a simpler Node and reduces the heap (tree) building time by 75% and
dramatically improves the time on small messages.

#### Key Improvements:

- **Array-based traversal:** reduces upfront costs of tree building.

#### Safety:

- **Iteration**: The safe version uses an iterator while the unsafe version uses
  a simple `loop` and dereferencing of const ptrs.


### Table:

This approach uses a multi-symbol lookup table built upfront by decoding all
8-bit paths through the tree.

Using the flat decoder to decode integers `0..=255` and storing the symbols
traversed and number of bits used in their own flat array creates a multi-symbol
lookup table that provides excellent data characteristics for the compiler and
the cpu.

#### Key Improvements:

- **[Bitter](https://github.com/nickbabcock/bitter):** for
  [blazing fast bit reading](https://github.com/nickbabcock/bitter#comparison-to-other-libraries).
- **Multi-symbol:** flat lookup table for each 8 step path from `0..=255`
  generated using the tree provided by `FlatNode`.
- **Loop unrolling:** manually for both bulk ('outer') and compiler unrolled for
  the per symbol ('inner') loops. With the inner's compiled code exactly
  matching a manually unrolled and optimized loop.

#### Safety:

- **Iteration:** on both the fully safe and safe except const pointer deref
  versions use the same safe decoding process, the difference is in the tree
  building. The unsafe version uses mut ptr writes and unchecked reads.
- **Bit buffering:** is fully checked in the safe version and unchecked for all
  bits except the tail in the unsafe version.


### FSM:

This approach uses a multi-table state machine built upfront by decoding all
8-bit paths through the tree for all internal nodes.

The upfront cost is high for this setup. The flat decoder is used to decode
a single path from a leaf node's parent through all 8 bit paths which is then
used to populate all leaf parent state tables, the remainder of the internal
nodes have their paths decoded and states recorded.

The decoding is done a full byte at a time. In the 2-channel versions the first
and second halves are processed together and the joined once the first half
converges with the second.

#### Key Improvements:

- **[Bitter](https://github.com/nickbabcock/bitter):** for
  [blazing fast bit reading](https://github.com/nickbabcock/bitter#comparison-to-other-libraries).
- **Multi-symbol finite state machine:** flat lookup table for each 8 step path
  from `0..=255` for each internal node generated using the `FlatNode` tree.
- **Loop unrolling:** manually for both bulk ('outer') and compiler unrolled for
  the per symbol ('inner') loops. With the inner's compiled code exactly
  matching a manually unrolled and optimized loop.

#### Safety:

- **Iteration:** The unsafe version uses mut ptr writes and unchecked reads and
  unsafe copy methods where beneficial.
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
