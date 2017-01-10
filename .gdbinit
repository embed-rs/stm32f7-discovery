define reset
    # reset board by setting the SYSRESETREQ bit it the AIRCR register
    # see http://infocenter.arm.com/help/index.jsp?topic=/com.arm.doc.dui0552a/Cihehdge.html
    set *(0xE000ED0C) = 0x05fa0004
end

define load-reset
    load
    reset
end

define lr
    load-reset
end

define lrc
    load
    reset
    continue
end

source semihosting.py
catch signal SIGTRAP
commands
  silent
  if (*(int)$pc&0xff) == 0xab
    pi SemiHostHelper.on_break()
    set $pc = $pc + 2
    continue
  else
    echo \n
    frame
  end
end
