import os
import sys
import random
import struct
from overrides import override

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.dirname(source_dir)
sys.path[0] = root_dir

class Rng():
    def __init__(self, seed):
        self.seed = seed

    def get_int(self, nbits):
        assert nbits > 0
        return 0

    def get_choice(self, n):
        assert n > 0
        return 0
    
    def get_choice_prob(self, prob):
        assert prob >= 0 and prob <= 1
        return False
    
    def get_choice_exp(self):
        return 0
    
    def get_choice_arr(self, arr):
        assert len(arr) > 0
        return arr[0]
    
    def get_float(self):
        return 0.0

class RandomRng(Rng):
    def __init__(self, seed):
        super().__init__(seed)
        self.rand = random.Random(seed)
    
    @override
    def get_int(self, nbits):
        assert nbits > 0
        return self.rand.randint(0, (1 << nbits) - 1)
    
    @override
    def get_choice(self, n):
        assert n > 0
        return self.rand.randint(0, n-1)
    
    @override
    def get_choice_prob(self, prob):
        assert prob >= 0 and prob <= 1
        return self.rand.random() < prob
    
    @override
    def get_choice_exp(self):
        choice = 0
        while self.rand.randint(0, 1) % 2 == 1:
            choice += 1
        return choice
    
    @override
    def get_choice_arr(self, arr):
        assert len(arr) > 0
        return self.rand.choice(arr)
    
    @override
    def get_float(self):
        val = struct.unpack('f', self.rand.randbytes(4))[0] # float range
        return val


class ConsumeRng(Rng):
    def __init__(self, seed):
        super().__init__(seed)

    def consume_seed(self, nbytes):
        if len(self.seed) >= nbytes:
            val = self.seed[-nbytes:]
            self.seed = self.seed[:-nbytes]
            return val
        else:
            arr = b'\x00'*(nbytes - len(self.seed)) + self.seed
            self.seed = b''
            return arr

    @override
    def get_int(self, nbits):
        assert nbits > 0
        num_bytes = (nbits + 7) // 8
        val = int.from_bytes(self.consume_seed(num_bytes), 'big')
        bitmask = (1 << nbits) - 1
        return val & bitmask # when seed is depleted, 0

    # does not contain n
    @override
    def get_choice(self, n):
        assert n > 0
        return self.get_int((n - 1).bit_length()) % n # when seed is depleted, 0
    
    @override
    def get_choice_prob(self, prob):
        assert prob >= 0 and prob <= 1
        val = self.get_int(23) / (2**23) # mantissa of 32-bit is 23 bits
        return val >= 1 - prob # when seed is depleted, False
    
    @override
    def get_choice_exp(self):
        choice = 0
        while self.get_int(1) == 1:
            choice += 1
        return choice # when seed is depleted, 0
    
    @override
    def get_choice_arr(self, arr):
        assert len(arr) > 0
        return arr[self.get_choice(len(arr))] # when seed is depleted, first element
    
    @override
    def get_float(self):
        val = struct.unpack('f', self.consume_seed(4))[0] # float range
        return val # when seed is depleted, 0.0
