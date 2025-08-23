# turbo-blif

low-memory-usage BLIF (berkeley logic interchange format) parser and writer.

supports:
- the latest BLIF specification (dated July 28, 1992)
- all yosys BLIF extensions
  (supports reading of BLIF files generated with `write_blif -iname -iattr -param -cname -blackbox -attr -conn -icells`)
- KISS state machines (which yosys doesn't even support)
- clock and delay constraints (yosys just ignores those)

If you found a program that generates non-standard BLIF attributes or keywords, please open a GitHub issue.
We want to support all non-standard extensions.
