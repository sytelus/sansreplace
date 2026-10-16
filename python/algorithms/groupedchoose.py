"""Grouped Cardchoose: exact sampling with expected O(k) integer operations.

Python integers have no fixed width limit. The output is the only array; the
sampler uses O(1) auxiliary integers. See groupedchoose.md for the probability,
work and overwrite proofs, and for the distinction between word and bit costs.
"""

import random


def _urn_into(out, start, count, low, width, draw):
    stop = start + count
    if width == 1:
        for i in range(start, stop):
            out[i] = low + i
        return
    for j in range(count):
        r = draw(width + j)
        out[start + j] = low + r if r < width else out[start + r - width]
    # A small, contiguous group normally fits in a few cache lines. The second
    # moment of the urn occupancies bounds the expected total insertion work.
    for i in range(start + 1, stop):
        value, j = out[i], i
        while j > start and out[j - 1] > value:
            out[j] = out[j - 1]
            j -= 1
        out[j] = value
    for i in range(start, stop):
        out[i] += i


def _emit_records(out, k, read, t, width, extra, draw):
    # Emit from the left. Each record consumes no more space than the output
    # it describes, so output cannot reach a record we have not yet read.
    start = 0
    while read < k:
        tag = out[read]
        read += 1
        if tag < t:
            out[start] = tag + start
            start += 1
        else:
            count, b = tag - t + 2, out[read]
            read += 1
            assert start + count <= read
            _urn_into(out, start, count, b * width + min(b, extra),
                      width + (b < extra), draw)
            start += count
    assert start == k


def _sparse(n, k, draw, group_size=16, out=None):
    if not k:
        return []
    t = n - k + 1
    groups = min(t, 1 + (k - 1) // group_size)
    if out is None:
        out = [0] * k
    if groups == 1:
        _urn_into(out, 0, k, 0, t, draw)
        return out
    assert 2 * groups <= k
    width, extra = divmod(t, groups)
    split = (width + 1) * extra

    # One-based labels; the first 'extra' groups have one more initial color.
    # All products below fit in the same integer width as n.
    for i in range(k):
        r = draw(t + i)
        if r < t:
            b = r // (width + 1) if r < split else extra + (r - split) // width
            out[i] = b + 1
        else:
            out[i] = out[r - t]

    # In-place histogram: zero is empty, 1..groups are labels, and groups+m
    # encodes a count m. Moving a label never discards an uncounted label.
    i = 0
    while i < k:
        if not 1 <= out[i] <= groups:
            i += 1
            continue
        b = out[i] - 1
        if 1 <= out[b] <= groups:
            out[i] = out[b]
            out[b] = groups + 1
        else:
            out[b] = groups + 1 if out[b] == 0 else out[b] + 1
            out[i] = 0
            i += 1

    # Pack at the right: a singleton is its color; a larger group is a
    # (t+count-2, group) pair. At most two slots per unread group are needed.
    read = k
    for b in range(groups - 1, -1, -1):
        count = out[b] - groups if out[b] else 0
        if count == 1:
            read -= 1
            out[read] = b * width + min(b, extra) + draw(width + (b < extra))
        elif count > 1:
            read -= 2
            out[read], out[read + 1] = t + count - 2, b
        assert read >= b

    _emit_records(out, k, read, t, width, extra, draw)
    return out


def _complement(n, k, draw, group_size=16):
    omitted = n - k
    assert 0 < omitted <= k // 4
    out = [0] * k
    _sparse(n, omitted, draw, group_size, out)
    t, read, high = omitted + 1, k, n
    for b in range(omitted, -1, -1):
        low = out[b - 1] + 1 if b else 0
        count, high = high - low, low - 1
        if count == 1:
            read -= 1
            out[read] = b
        elif count > 1:
            read -= 2
            out[read], out[read + 1] = t + count - 2, b
        assert read >= b
    _emit_records(out, k, read, t, 1, 0, draw)
    return out


def sorted_choose(n, k):
    """Return an exactly uniform sorted k-subset of range(n), for 0 <= k <= n."""
    if not 0 <= k <= n:
        raise ValueError('require 0 <= k <= n')
    if not k:
        return []
    if k == n:
        return list(range(n))
    if n - k == 1:
        omitted = random.randrange(n)
        return [i + (i >= omitted) for i in range(k)]
    if n - k <= k // 4:
        return _complement(n, k, random.randrange)
    if n - k > k:
        return _sparse(n, k, random.randrange)
    out = [0] * k
    written = 0
    for value in range(n):
        if random.randrange(n - value) < k - written:
            out[written] = value
            written += 1
    return out


def random_choose(n, k):
    """Return a uniform ordered sample; every k-permutation is equally likely."""
    if not 0 <= k <= n:
        raise ValueError('require 0 <= k <= n')
    if not k:
        return []
    if n - k <= k:
        # Reservoir sampling has O(k) work only in this dense regime. Its
        # initial inside-out shuffle makes the order uniform as well.
        out = [0] * k
        for i in range(k):
            j = random.randrange(i + 1)
            out[i] = out[j]
            out[j] = i
        for i in range(k, n):
            j = random.randrange(i + 1)
            if j < k:
                out[j] = i
        return out
    out = _sparse(n, k, random.randrange)
    for i in range(1, k):
        j = random.randrange(i + 1)
        out[i], out[j] = out[j], out[i]
    return out
