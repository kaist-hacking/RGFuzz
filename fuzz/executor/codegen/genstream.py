import os
import sys
import itertools
import random

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = source_dir
sys.path[0] = root_dir

import config

exhaustive_seed_gen = itertools.count(config.codegen_seed_enum_start_from)
random_seed_gen = map(random.randint, itertools.repeat(0), itertools.repeat(2**(config.codegen_seed_len*8)-1))
code_seed_gen = random_seed_gen if config.codegen_is_random else exhaustive_seed_gen
