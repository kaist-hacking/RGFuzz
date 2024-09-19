#!/usr/bin/env python3

import os
import sys

source_dir = os.path.dirname(os.path.abspath(__file__))

if __name__ == "__main__":
    v8_path = os.path.join(source_dir, "v8")
    gm_py_path = os.path.join(v8_path, "tools", "dev", "gm.py")
    with open(gm_py_path, "rt") as f:
        gm_py_content = f.read()

    template_idx = gm_py_content.find("RELEASE_ARGS_TEMPLATE")
    assert template_idx >= 0
    template_start_idx = gm_py_content.find('"""', template_idx)
    assert template_start_idx >= template_idx
    template_end_idx = gm_py_content.find('"""', template_start_idx+1)
    assert template_end_idx > template_start_idx

    gm_py_content = gm_py_content[:template_end_idx] + \
        "use_clang_coverage = true\n" + \
        gm_py_content[template_end_idx:]

    new_gm_py_path = os.path.join(v8_path, "tools", "dev", "gm_modi.py")
    with open(new_gm_py_path, "wt") as f:
        f.write(gm_py_content)
