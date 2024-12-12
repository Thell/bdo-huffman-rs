# ** 🚧 UNDER CONSTRUCTION 🚧 **

# BDO Huffman Decoder

This project explores implementing a specific
[Huffman encoding](https://en.wikipedia.org/wiki/Huffman_coding) decoder to
decompress packet contents in responses from calls to BDO's GetWorldMarket
endpoints.

A few different approaches of decoding the Huffman message will be explored.

- tree traversal
- single symbol table lookup
- multiple symbol table lookup
- single symbol SIMD processing

## Usage
Clone the repo and run any of:

- `cargo test`
- `cargo bench`
- `cargo run --example`

A few examples are included for profiling and summary information dumping.

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

The symbol table can consist of `-`, `0-9` and `|`. They are always in the same
order (ASCII value order) and if a symbol doesn't occur in the message it is
omitted from the symbol table.

Once decoded '|' and '-' denote record and field delimiters respectively, they
will always be present in the table. There are four fields in each record:
'item', 'count', 'price' and 'cumulative count'. This repo is only concerned
with decoding so parsing the decoded message is out of scope but if someone was
parsing the decoded messages (perhaps to feed into a database) then
incorporating the record structure and emitting an update message into the
decoder should be very efficient vs post processing.

#### A note on the symbol table and prefix lengths.

When parsing the packet ensure that whatever container is used to store the
symbol and frequency has stable order! The input order is crucial to the proper
building of the Huffman tree.

The minimum prefix len is 1, the maximum observed prefix is 7.
(My testing with heavily skewed frequencies never resulted in a prefix greater
than 7 but to be safe imagine 8 _could_ be possible.)

### The Heap

Since the incoming messages are already encoded we must use a Min-Heap that
orders the symbol nodes the same way to decode them. BDO's encoding
matches the ordering of the the classic Min-BinaryHeap. Many modern heap
implementations are optimized and do not provide the ordering required and if
they do provide the right ordering they may be sub-optimal for this use case.

What you want is a simple collection and simple _sift-up_ and _sift-down_
procedures implemented directly as _push_ and _pop_ functions respectively.

An example in python pseudo code where `heap` os a simple list and `node` is a
simple object containing `['symbol', 'freq', 'left_child', 'right_child']`

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

|Group        | Main | Sub | Decoded Size  |
|:------------|------|-----|---------------|
|large        | 55   | 4   |70.5k          |
|large_medium | 55   | 3   |33.3k          |
|medium       | 55   | 2   |22.5k          |
|medium_small | 55   | 1   |11.1k          |
|small        | 25   | 2   |5.5k           |
|small_min    | 75   | 6   |40b            |

While the message size is not exactly tied to the number of items in a category
it is directly tied to it.

 _(See
[bdoMarket Master Items Table](
https://docs.google.com/spreadsheets/d/1LFri67Eb2nW8VmoG7FGNXIhGAexqGxdZnNQqvzCm-dw
) for categorized data to analyze.)_

## Decoders

In the world of data compression the message sizes are miniscule. This means
many of the algorithms and optimizations used in decompression libraries are not
beneficial since any overhead in the preparation of the decoding would have a
difficult time being amortized away before decoding is completed. Also, since
the symbols are dynamic and the encoding scheme is set any optimizations that
require specific encoding or embeddings are also not useful.
