import struct

def packf64(x):
    return struct.pack('<d', x)


def packf32(x):
    return struct.pack('<f', x)


def packu32(x):
    return struct.pack('<I', x)


def packstr(x):
    bb = x.encode('utf-8')
    return packvu32(len(bb)) + bb


def packvs64(x):
    bb = signed_leb128_encode(x)
    # assert len(bb) <= 8
    return bb


def packvs32(x):
    bb = signed_leb128_encode(x)
    # assert len(bb) <= 4
    return bb

def packvu32(x):
    bb = unsigned_leb128_encode(x)
    # assert len(bb) <= 4
    return bb


def packvs33(x):
    bb = signed_leb128_encode(x)
    return bb


def packvu7(x):
    bb = unsigned_leb128_encode(x)
    assert len(bb) == 1
    return bb


def packvu8(x):
    bb = unsigned_leb128_encode(x)
    # assert len(bb) == 1
    return bb


def packvu1(x):
    bb = unsigned_leb128_encode(x)
    assert len(bb) == 1
    return bb


def signed_leb128_encode(value):
    bb = []
    if value < 0:
        unsignedRefValue = (1 - value) * 2
    else:
        unsignedRefValue = value * 2
    while True:
        byte = value & 0x7F
        value >>= 7
        unsignedRefValue >>= 7
        if unsignedRefValue != 0:
            byte = byte | 0x80
        bb.append(byte)
        if unsignedRefValue == 0:
            break
    return bytes(bb)


def unsigned_leb128_encode(value):
    bb = []  # ints, really
    while True:
        byte = value & 0x7F
        value >>= 7
        if value != 0:
            byte = byte | 0x80
        bb.append(byte)
        if value == 0:
            break
    return bytes(bb)