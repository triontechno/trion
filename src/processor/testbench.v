// Self-checking TRION-H binary processor testbench.
// Run only on the explicit "TEST IT" instruction.

module testbench;
    reg clk;
    reg reset;

    wire halted;
    wire fault;
    wire [3:0] fault_code;
    wire [63:0] fault_pc;
    wire [31:0] fault_instruction;
    wire [63:0] pc;
    wire instruction_retired;
    wire [63:0] retired_count;

    integer errors;
    integer index;
    integer cycles;

    reg [127:0] alu_a;
    reg [127:0] alu_b;
    reg [3:0] alu_operation;
    wire [127:0] alu_result;
    wire alu_valid;

    localparam OP_ADD  = 7'h01;
    localparam OP_SUB  = 7'h02;
    localparam OP_AND  = 7'h03;
    localparam OP_OR   = 7'h04;
    localparam OP_XOR  = 7'h05;
    localparam OP_SLL  = 7'h06;
    localparam OP_SRL  = 7'h07;
    localparam OP_ADDI = 7'h08;
    localparam OP_LD   = 7'h09;
    localparam OP_ST   = 7'h0A;
    localparam OP_BEQ  = 7'h0B;
    localparam OP_BNE  = 7'h0C;
    localparam OP_JAL  = 7'h0D;
    localparam OP_HALT = 7'h7E;

    localparam FAULT_INSTRUCTION_RANGE = 4'd2;
    localparam FAULT_ILLEGAL_OPCODE = 4'd3;
    localparam FAULT_DATA_ALIGNMENT = 4'd4;
    localparam FAULT_DATA_RANGE = 4'd5;

    trion_h_alu128 alu_unit (
        .operand_a(alu_a),
        .operand_b(alu_b),
        .operation(alu_operation),
        .result(alu_result),
        .valid_operation(alu_valid)
    );

    trion_h_sim_system dut (
        .clk(clk),
        .reset(reset),
        .halted(halted),
        .fault(fault),
        .fault_code(fault_code),
        .fault_pc(fault_pc),
        .fault_instruction(fault_instruction),
        .pc(pc),
        .instruction_retired(instruction_retired),
        .retired_count(retired_count)
    );

    always #5 clk = ~clk;

    function [31:0] enc;
        input [6:0] operation;
        input [4:0] destination;
        input [4:0] source_a;
        input [4:0] source_b;
        input [9:0] imm;
        begin
            enc = {operation, destination, source_a, source_b, imm};
        end
    endfunction

    task check;
        input condition;
        input [8*96-1:0] label;
        begin
            if (condition) begin
                $display("PASS: %0s", label);
            end else begin
                $display("FAIL: %0s", label);
                errors = errors + 1;
            end
        end
    endtask

    task tick;
        begin
            @(posedge clk);
            #1;
        end
    endtask

    task reset_machine;
        begin
            reset = 1'b1;
            repeat (2) tick;
            reset = 1'b0;
            #1;
        end
    endtask

    task clear_memories;
        begin
            for (index = 0; index < 256; index = index + 1) begin
                dut.imem[index] = enc(OP_HALT, 5'd0, 5'd0, 5'd0, 10'd0);
                dut.dmem[index] = 128'd0;
            end
        end
    endtask

    task run_until_stop;
        input integer limit;
        begin
            cycles = 0;
            while ((cycles < limit) && !halted && !fault) begin
                tick;
                cycles = cycles + 1;
            end
        end
    endtask

    task reset_program;
        begin
            clear_memories;
            reset_machine;
        end
    endtask

    initial begin
        clk = 1'b0;
        reset = 1'b1;
        errors = 0;
        cycles = 0;
        alu_a = 128'd0;
        alu_b = 128'd0;
        alu_operation = 4'd0;

        clear_memories;
        reset_machine;

        // -------------------------------------------------------------
        // Standalone full-width binary ALU coverage.
        // -------------------------------------------------------------
        alu_a = 128'd100;
        alu_b = 128'd7;

        alu_operation = 4'd0; #1;
        check(alu_valid && alu_result == 128'd107, "ALU ADD");
        alu_operation = 4'd1; #1;
        check(alu_valid && alu_result == 128'd93, "ALU SUB");
        alu_operation = 4'd2; #1;
        check(alu_valid && alu_result == 128'd4, "ALU AND");
        alu_operation = 4'd3; #1;
        check(alu_valid && alu_result == 128'd103, "ALU OR");
        alu_operation = 4'd4; #1;
        check(alu_valid && alu_result == 128'd99, "ALU XOR");
        alu_operation = 4'd5; #1;
        check(alu_valid && alu_result == 128'd12800, "ALU SLL");
        alu_operation = 4'd6; #1;
        check(alu_valid && alu_result == 128'd0, "ALU SRL");
        alu_a = {128{1'b1}};
        alu_b = 128'd1;
        alu_operation = 4'd0; #1;
        check(alu_valid && alu_result == 128'd0, "ALU 128-bit ADD wraparound");
        alu_a = 128'd0;
        alu_b = 128'd1;
        alu_operation = 4'd1; #1;
        check(alu_valid && alu_result == {128{1'b1}}, "ALU 128-bit SUB wraparound");
        alu_operation = 4'd15; #1;
        check(!alu_valid, "ALU rejects unsupported control encoding");

        // -------------------------------------------------------------
        // Binary instruction execution, memory, control flow, retirement.
        // -------------------------------------------------------------
        reset_program;
        dut.imem[0]  = enc(OP_ADDI, 5'd1,  5'd0, 5'd0, 10'd100);
        dut.imem[1]  = enc(OP_ADDI, 5'd2,  5'd0, 5'd0, 10'd7);
        dut.imem[2]  = enc(OP_ADD,  5'd3,  5'd1, 5'd2, 10'd0);
        dut.imem[3]  = enc(OP_SUB,  5'd4,  5'd3, 5'd2, 10'd0);
        dut.imem[4]  = enc(OP_AND,  5'd5,  5'd1, 5'd2, 10'd0);
        dut.imem[5]  = enc(OP_OR,   5'd6,  5'd1, 5'd2, 10'd0);
        dut.imem[6]  = enc(OP_XOR,  5'd7,  5'd1, 5'd2, 10'd0);
        dut.imem[7]  = enc(OP_ADDI, 5'd8,  5'd0, 5'd0, 10'd3);
        dut.imem[8]  = enc(OP_SLL,  5'd9,  5'd1, 5'd8, 10'd0);
        dut.imem[9]  = enc(OP_SRL,  5'd10, 5'd1, 5'd8, 10'd0);
        dut.imem[10] = enc(OP_ST,   5'd0,  5'd0, 5'd3, 10'd16);
        dut.imem[11] = enc(OP_LD,   5'd11, 5'd0, 5'd0, 10'd16);
        dut.imem[12] = enc(OP_BEQ,  5'd0,  5'd3, 5'd11, 10'd2);
        dut.imem[13] = enc(OP_ADDI, 5'd12, 5'd0, 5'd0, 10'd999);
        dut.imem[14] = enc(OP_BNE,  5'd0,  5'd3, 5'd11, 10'd2);
        dut.imem[15] = enc(OP_JAL,  5'd13, 5'd0, 5'd0, 10'd2);
        dut.imem[16] = enc(OP_ADDI, 5'd14, 5'd0, 5'd0, 10'd777);
        dut.imem[17] = enc(OP_ADDI, 5'd14, 5'd0, 5'd0, 10'd42);
        dut.imem[18] = enc(OP_ADDI, 5'd15, 5'd1, 5'd0, 10'b1111111011);
        dut.imem[19] = enc(OP_ADDI, 5'd0,  5'd1, 5'd0, 10'd55);
        dut.imem[20] = enc(OP_HALT, 5'd0,  5'd0, 5'd0, 10'd0);

        reset_machine;
        run_until_stop(64);

        check(!fault, "main program completes without a fault");
        check(halted, "HALT stops execution");
        check(dut.cpu.register_file.registers[1] == 128'd100, "ADDI positive immediate");
        check(dut.cpu.register_file.registers[2] == 128'd7, "ADDI second source");
        check(dut.cpu.register_file.registers[3] == 128'd107, "register ADD");
        check(dut.cpu.register_file.registers[4] == 128'd100, "register SUB");
        check(dut.cpu.register_file.registers[5] == 128'd4, "register AND");
        check(dut.cpu.register_file.registers[6] == 128'd103, "register OR");
        check(dut.cpu.register_file.registers[7] == 128'd99, "register XOR");
        check(dut.cpu.register_file.registers[9] == 128'd800, "register SLL");
        check(dut.cpu.register_file.registers[10] == 128'd12, "register SRL");
        check(dut.dmem[1] == 128'd107, "ST writes one 128-bit word");
        check(dut.cpu.register_file.registers[11] == 128'd107, "LD reads one 128-bit word");
        check(dut.cpu.register_file.registers[12] == 128'd0, "taken BEQ skips the next instruction");
        check(dut.cpu.register_file.registers[13] == 128'd64, "JAL writes PC plus four");
        check(dut.cpu.register_file.registers[14] == 128'd42, "JAL skips the sequential instruction");
        check(dut.cpu.register_file.registers[15] == 128'd95, "ADDI sign-extends a negative immediate");
        check(dut.cpu.register_file.registers[0] == 128'd0, "register zero remains hardwired to zero");
        check(retired_count == 64'd19, "retirement counter includes HALT and excludes skipped instructions");
        check(pc == 64'd80, "PC remains on the HALT instruction");

        // -------------------------------------------------------------
        // Taken BNE and ordinary not-taken BEQ.
        // -------------------------------------------------------------
        reset_program;
        dut.imem[0] = enc(OP_ADDI, 5'd1, 5'd0, 5'd0, 10'd1);
        dut.imem[1] = enc(OP_ADDI, 5'd2, 5'd0, 5'd0, 10'd2);
        dut.imem[2] = enc(OP_BEQ,  5'd0, 5'd1, 5'd2, 10'd2); // not taken
        dut.imem[3] = enc(OP_BNE,  5'd0, 5'd1, 5'd2, 10'd2); // taken to 5
        dut.imem[4] = enc(OP_ADDI, 5'd3, 5'd0, 5'd0, 10'd77);
        dut.imem[5] = enc(OP_ADDI, 5'd3, 5'd0, 5'd0, 10'd9);
        dut.imem[6] = enc(OP_HALT, 5'd0, 5'd0, 5'd0, 10'd0);
        reset_machine;
        run_until_stop(24);
        check(halted && !fault, "branch program halts");
        check(dut.cpu.register_file.registers[3] == 128'd9, "taken BNE skips branch-delay-free sequential instruction");

        // -------------------------------------------------------------
        // Misaligned load and store must fault without retiring.
        // -------------------------------------------------------------
        reset_program;
        dut.imem[0] = enc(OP_ADDI, 5'd1, 5'd0, 5'd0, 10'd4);
        dut.imem[1] = enc(OP_LD, 5'd2, 5'd1, 5'd0, 10'd0);
        reset_machine;
        run_until_stop(8);
        check(fault && fault_code == FAULT_DATA_ALIGNMENT, "misaligned LD raises data-alignment fault");
        check(fault_pc == 64'd4 && retired_count == 64'd1, "misaligned LD does not retire");

        reset_program;
        dut.imem[0] = enc(OP_ADDI, 5'd1, 5'd0, 5'd0, 10'd4);
        dut.imem[1] = enc(OP_ST, 5'd0, 5'd1, 5'd0, 10'd0);
        reset_machine;
        run_until_stop(8);
        check(fault && fault_code == FAULT_DATA_ALIGNMENT, "misaligned ST raises data-alignment fault");
        check(retired_count == 64'd1, "misaligned ST does not retire");

        // -------------------------------------------------------------
        // Illegal opcode, memory range, and instruction range faults.
        // -------------------------------------------------------------
        reset_program;
        dut.imem[0] = enc(7'h10, 5'd1, 5'd0, 5'd0, 10'd0);
        reset_machine;
        run_until_stop(4);
        check(fault && fault_code == FAULT_ILLEGAL_OPCODE, "reserved opcode raises illegal-instruction fault");
        check(retired_count == 64'd0, "illegal instruction is not retired");

        reset_program;
        dut.imem[0] = enc(OP_ADDI, 5'd1, 5'd0, 5'd0, 10'd1);
        dut.imem[1] = enc(OP_ADDI, 5'd2, 5'd0, 5'd0, 10'd12);
        dut.imem[2] = enc(OP_SLL,  5'd3, 5'd1, 5'd2, 10'd0); // address 4096
        dut.imem[3] = enc(OP_LD,   5'd4, 5'd3, 5'd0, 10'd0);
        reset_machine;
        run_until_stop(12);
        check(fault && fault_code == FAULT_DATA_RANGE, "out-of-range LD raises data-range fault");
        check(fault_pc == 64'd12 && retired_count == 64'd3, "out-of-range LD does not retire");

        reset_program;
        dut.imem[0] = enc(OP_JAL, 5'd0, 5'd0, 5'd0, 10'd1023);
        reset_machine;
        run_until_stop(8);
        check(fault && fault_code == FAULT_INSTRUCTION_RANGE, "out-of-range instruction fetch faults");
        check(fault_pc == 64'd4092 && retired_count == 64'd1, "instruction fetch range fault records PC");

        // -------------------------------------------------------------
        // Reset clears sticky terminal state and architectural PC/count.
        // -------------------------------------------------------------
        reset_machine;
        check(!fault && !halted && pc == 64'd0 && retired_count == 64'd0,
              "reset clears halt, fault, PC, and retirement count");

        if (errors == 0)
            $display("ALL TRION-H BINARY PHASE 2 TESTS PASSED");
        else
            $display("TRION-H BINARY PHASE 2 FAILED: %0d check(s)", errors);
        $finish;
    end
endmodule
