\`timescale 1ns/1ps

module testbench;
    reg clk=0;
    always #5 clk=~clk;
    reg reset;
    wire halted, fault;
    wire [63:0] pc;
    wire [31:0] last_instruction;
    wire instruction_retired;
    integer errors;
    reg [127:0] alu_a_check, alu_b_check;
    reg [3:0] alu_op_check;
    wire [127:0] alu_result_check;
    wire alu_valid_check;

    trion_h_alu128 alu_check(
        .operand_a(alu_a_check), .operand_b(alu_b_check),
        .operation(alu_op_check), .result(alu_result_check),
        .valid_operation(alu_valid_check)
    );

    trion_h_core dut(
        .clk(clk), .reset(reset), .halted(halted), .fault(fault),
        .pc(pc), .last_instruction(last_instruction),
        .instruction_retired(instruction_retired)
    );

    localparam OP_ADD=7'h01, OP_SUB=7'h02, OP_XOR=7'h05,
               OP_ADDI=7'h08, OP_LD=7'h09, OP_ST=7'h0A,
               OP_BEQ=7'h0B, OP_BNE=7'h0C, OP_JAL=7'h0D,
               OP_TADD=7'h10, OP_TSUB=7'h11, OP_HALT=7'h7E;

    function [31:0] enc;
        input [6:0] op;
        input [4:0] rd, rs1, rs2;
        input [9:0] imm;
        begin enc={op,rd,rs1,rs2,imm}; end
    endfunction

    task check;
        input condition;
        input [255:0] label;
        begin
            if(condition) $display("PASS: %0s",label);
            else begin $display("FAIL: %0s",label); errors=errors+1; end
        end
    endtask

    task tick;
        begin @(posedge clk); #1; end
    endtask

    integer j;
    initial begin
        errors=0;
        reset=1;
        for(j=0;j<256;j=j+1) begin
            dut.imem[j]=enc(OP_HALT,0,0,0,0);
            dut.dmem[j]=0;
        end
        repeat(2) tick();
        reset=0;

        // Program: immediate, register ALU, memory, branch, jump, ternary.
        dut.imem[0]  = enc(OP_ADDI,1,0,0,10'd100);
        dut.imem[1]  = enc(OP_ADDI,2,0,0,10'd23);
        dut.imem[2]  = enc(OP_ADD,3,1,2,0);
        dut.imem[3]  = enc(OP_SUB,4,3,2,0);
        dut.imem[4]  = enc(OP_ST,0,0,3,10'd16);
        dut.imem[5]  = enc(OP_LD,5,0,0,10'd16);
        dut.imem[6]  = enc(OP_BEQ,0,3,5,10'd2);
        dut.imem[7]  = enc(OP_ADDI,6,0,0,10'd999);
        dut.imem[8]  = enc(OP_XOR,7,1,2,0);
        dut.imem[9]  = enc(OP_JAL,8,0,0,10'd2);
        dut.imem[10] = enc(OP_ADDI,9,0,0,10'd777);
        dut.imem[11] = enc(OP_TADD,10,11,12,0);
        dut.imem[12] = enc(OP_TSUB,13,10,12,0);
        dut.imem[13] = enc(OP_HALT,0,0,0,0);

        dut.gpr[11]=128'h0;
        dut.gpr[11][1:0]=2'b01;
        dut.gpr[11][3:2]=2'b00;
        dut.gpr[11][5:4]=2'b10;
        dut.gpr[12]=128'h0;
        dut.gpr[12][1:0]=2'b01;
        dut.gpr[12][3:2]=2'b01;

        for(j=0;j<20 && !halted && !fault;j=j+1) tick();

        check(!fault,"no illegal instruction or alignment fault");
        check(halted,"HALT instruction stops processor");
        check(dut.gpr[1]==128'd100,"ADDI writes r1");
        check(dut.gpr[2]==128'd23,"ADDI writes r2");
        check(dut.gpr[3]==128'd123,"128-bit ADD writes r3");
        check(dut.gpr[4]==128'd100,"128-bit SUB writes r4");
        check(dut.dmem[1]==128'd123,"128-bit STORE writes memory");
        check(dut.gpr[5]==128'd123,"128-bit LOAD reads memory");
        check(dut.gpr[6]==128'd0,"taken BEQ skips instruction");
        check(dut.gpr[7]==(128'd100 ^ 128'd23),"128-bit XOR executes");
        check(dut.gpr[8]==128'd40,"JAL writes return address");
        check(dut.gpr[9]==128'd0,"JAL skips sequential instruction");
        check(dut.gpr[10][1:0]==2'b10 && dut.gpr[10][3:2]==2'b10 && dut.gpr[10][5:4]==2'b00,
              "packed balanced-ternary ADD");
        check(dut.gpr[13][1:0]==2'b01 && dut.gpr[13][3:2]==2'b00 && dut.gpr[13][5:4]==2'b10,
              "packed balanced-ternary SUB");
        check(dut.gpr[0]==0,"R0 remains hardwired to zero");

        alu_a_check=128'd9;
        alu_b_check=128'd4;
        alu_op_check=4'd15;
        #1;
        check(!alu_valid_check,"ALU rejects unsupported operation");

        if(errors==0) $display("ALL TRION-H PHASE 1 TESTS PASSED");
        else $display("TRION-H PHASE 1 FAILED: %0d checks",errors);
        $finish;
    end
endmodule
