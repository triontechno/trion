# TRION-H binary smoke program
# r1 = 1
# r2 = 2
# r3 = r1 + r2  (expected 3)
# Stop
ADDI r1, r0, 1
ADDI r2, r0, 2
ADD  r3, r1, r2
HALT
