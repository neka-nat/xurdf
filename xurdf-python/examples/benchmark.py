import sys
sys.path.insert(0, "../../tmp/xacro/src")
import time
import xurdfpy
import xacro

start = time.time()
for _ in range(100):
    doc = xurdfpy.parse_xacro_file("../../data/sample.xacro")
end = time.time()
print("xurdfpy: ", end - start)
print(doc)

start = time.time()
for _ in range(100):
    doc = xacro.process_file("../../data/sample.xacro")
    doc = doc.toprettyxml(indent='  ')
end = time.time()
print("xacro: ", end - start)
print(doc)
