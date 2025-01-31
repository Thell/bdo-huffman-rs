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
| Original          |   ✅    |  Box  |      -       |   398.4 µs   |   177.2 µs   |   113.0 µs   |   53.59 µs   |   38.98 µs   |   1.375 µs   |
| BaseLine          |   ✅    |  Box  |      -       |   323.8 µs   |   154.2 µs   |   99.76 µs   |   43.35 µs   |   22.98 µs   |   497.7 ns   |
|                   |        |       |              |              |              |              |              |              |              |
| Nested            |   ✅    |  Box  |      -       |   297.9 µs   |   134.5 µs   |   78.06 µs   |   25.84 µs   |   17.14 µs   |   530.3 ns   |
| **Flat**          |   ✅    | Index |      -       |   298.2 µs   |   133.4 µs   |   76.98 µs   |   26.04 µs   |   17.34 µs   | **192.8** ns |
| S-Table           |   ✅    | Index |      -       |   141.9 µs   |   64.46 µs   |   43.06 µs   |   21.40 µs   |   10.50 µs   |   288.5 ns   |
| **M-Table**       |   ✅    | Index |   111.3 µs   |   56.69 µs   |   27.42 µs   | **18.76** µs | **9.961** µs | **6.630** µs |   1.912 µs   |
| FSM               |   ✅    | Index |   110.1 µs   |   60.01 µs   |   33.82 µs   |   25.60 µs   |   18.00 µs   |   15.80 µs   |   10.63 µs   |
| 2 Channel FSM     |   ✅    | Index |   83.75 µs   |   47.00 µs   |   27.86 µs   |   22.02 µs   |   16.86 µs   |   14.19 µs   |   11.24 µs   |
| **3 Channel FSM** |   ✅    | Index | **79.40** µs | **44.65** µs | **26.65** µs |   21.13 µs   |   15.91 µs   |   13.74 µs   |   10.84 µs   |
| 4 Channel FSM     |   ✅    | Index |   92.78 µs   |   50.47 µs   |   29.49 µs   |   22.83 µs   |   16.17 µs   |   14.28 µs   |   10.69 µs   |
|                   |        |       |              |              |              |              |              |              |              |
| Nested            |   ❌    |  Box  |      -       |   176.2 µs   |   62.02 µs   |   26.75 µs   |   13.38 µs   |   8.120 µs   |   439.8 ns   |
| **Flat**          |   ❌    | Const |      -       |   175.7 µs   |   61.08 µs   |   26.04 µs   |   12.85 µs   |   7.669 µs   | **146.4** ns |
| S-Table           |   ❌    | Const |      -       |   128.7 µs   |   58.37 µs   |   38.59 µs   |   19.21 µs   |   9.362 µs   |   256.4 ns   |
| **M-Table**       |   ❌    | Const |   102.9 µs   |   51.95 µs   |   24.69 µs   | **16.85** µs | **8.582** µs | **5.347** µs |   932.8 ns   |
| FSM               |   ❌    | Index |   97.44 µs   |   53.70 µs   |   30.56 µs   |   23.48 µs   |   16.84 µs   |   14.84 µs   |   10.12 µs   |
| 2 Channel FSM     |   ❌    | Index |   65.19 µs   |   37.35 µs   |   22.88 µs   |   18.36 µs   |   14.25 µs   |   12.95 µs   |   10.10 µs   |
| 3 Channel FSM     |   ❌    | Index |   58.04 µs   |   33.83 µs   |   21.17 µs   |   17.37 µs   |   13.59 µs   |   12.52 µs   |   10.16 µs   |
| **4 Channel FSM** |   ❌    | Index | **57.80** µs | **33.81** µs | **21.09** µs |   17.25 µs   |   13.57 µs   |   12.50 µs   |   10.18 µs   |
| 5 Channel FSM     |   ❌    | Index |   62.67 µs   |   36.08 µs   |   22.09 µs   |   18.12 µs   |   14.28 µs   |   12.74 µs   |   10.25 µs   |

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
