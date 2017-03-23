from __future__ import print_function
import gdb
import re
import struct
import sys


class SemiHostHelper(object):
    SANE_FDS = (1, 2)

    @classmethod
    def on_break(cls):
        # get the current frame and inferior
        frame = gdb.selected_frame()
        inf = gdb.selected_inferior()

        # retrieve instruction
        ins = frame.architecture().disassemble(frame.pc())[0]
        m = re.match(r'^bkpt\s+((?:0x)?[0-9a-f]+)$', ins['asm'].lower())

        if m:
            raw = m.group(1)
            # we've matched a breakpoint, decode the immediate
            bkpt_n = int(raw, 16 if raw.startswith('0x') else 10)

            # breakpoint 0xab indicates a semi-hosting call
            if bkpt_n == 0xAB:
                # retrieve the call type and obj registers
                # note: we would like to use `Frame.read_registers()`
                #       for this, but its only available on gdb 7.8 or
                #       newer
                r0 = gdb.parse_and_eval('$r0')
                r1 = gdb.parse_and_eval('$r1')

                call_type = int(r0)
                arg_addr = int(r1)

                if call_type == 0x05:
                    cls.handle_write(inf, arg_addr)
                else:
                    raise NotImplementedError(
                        'Call type 0x{:X} not implemented'
                        .format(call_type))
            else:
                raise ValueError('no semi-hosting breakpoint')
        else:
            raise ValueError('no bkpt instruction')

    @classmethod
    def handle_write(cls, inf, args_addr):
        # argument struct has three u32 entries: fd, address, len
        buf = inf.read_memory(args_addr, 12)

        fd, addr, l = struct.unpack('<lll', buf)

        # limit length to 4M to avoid funky behavior
        l = min(l, 4 * 1024 * 1024)

        # sanity check file descriptor
        if fd not in cls.SANE_FDS:
            raise ValueError(
                'Refusing to write to file descriptor {}'
                ' (not in {})'.format(fd, cls.SANE_FDS))

        # read the memory
        data = bytes(inf.read_memory(addr, l))

        # we manually map FDs. encoding is fixed at the rust-native utf8
        if fd == 1:
            sys.stdout.write(data.decode('utf8'))
        elif fd == 2:
            sys.stderr.write(data.decode('utf8'))
