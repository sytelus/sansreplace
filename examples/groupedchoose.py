def sorted_choose(n, k):
    """Uniform sorted k-subset; expected O(k) integer operations."""
    from random import randrange

    if not 0 <= k <= n:
        raise ValueError("require 0 <= k <= n")
    if k == n:
        return list(range(k))
    out = [0] * k
    if not k:
        return out
    if n <= 2 * k:
        written = 0
        for value in range(n):
            if randrange(n - value) < k - written:
                out[written] = value
                written += 1
                if written == k:
                    break
        return out

    def emit(start, count, low, width):
        for j in range(count):
            r = randrange(width + j)
            out[start + j] = (low + r if r < width
                              else out[start + r - width])
        for i in range(start + 1, start + count):
            value, j = out[i], i
            while j > start and out[j - 1] > value:
                out[j] = out[j - 1]
                j -= 1
            out[j] = value
        for i in range(start, start + count):
            out[i] += i

    t, groups = n - k + 1, 1 + (k - 1) // 16
    if groups == 1:
        emit(0, k, 0, t)
        return out
    width, extra = divmod(t, groups)
    split = (width + 1) * extra
    for i in range(k):
        r = randrange(t + i)
        if r < t:
            b = (r // (width + 1) if r < split
                 else extra + (r - split) // width)
            out[i] = b + 1
        else:
            out[i] = out[r - t]

    # Reuse the output prefix for group counts.
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
            out[b] = groups + 1 if not out[b] else out[b] + 1
            out[i] = 0
            i += 1

    # Pack records at the right, then emit at the left.
    read = k
    for b in range(groups - 1, -1, -1):
        count = out[b] - groups if out[b] else 0
        if count == 1:
            read -= 1
            out[read] = (b * width + min(b, extra)
                         + randrange(width + (b < extra)))
        elif count > 1:
            read -= 2
            out[read], out[read + 1] = t + count - 2, b
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
            emit(start, count, b * width + min(b, extra),
                 width + (b < extra))
            start += count
    return out
