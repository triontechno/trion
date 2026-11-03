// TRION-H binary processor RTL.
// Architecture: 32-bit instructions, 128-bit datapath, 32 x 128-bit GPRs,
// 64-bit byte-addressed PC. This implementation contains binary operations
// only. Unassigned opcodes are illegal instructions.
//
// Instruction encoding (all instructions):
//   [31:25] opcode | [24:20] rd | [19:15] rs1 | [14:10] rs2 | [9:0] imm10
//
// Memory contract:
//   Instruction port is combinational, addressed by byte PC.
//   Data port is combinational-read / edge-write, transfers one 128-bit word.
//   LD/ST effective addresses are byte addresses and must be 16-byte aligned.
//
// Synthesizable modules: decoder, ALU, register file, core.
// trion_h_sim_system and top are simulation/integration wrappers.

module trion_h_decoder (
    input  wire [31:0] instruction,
    output wire [6:0]  opcode,
    output wire [4:0]  rd,
    output wire [4:0]  rs1,
    output wire [4:0]  rs2,
    output wire [9:0]  immediate,
    output wire [127:0] immediate_sext
);
    assign opcode       = instruction[31:25];
    assign rd           = instruction[24:20];
    assign rs1          = instruction[19:15];
    assign rs2          = instruction[14:10];
    assign immediate    = instruction[9:0];
    assign immediate_sext = {{118{instruction[9]}}, instruction[9:0]};
endmodule


module trion_h_alu128 (
    input  wire [127:0] operand_a,
    input  wire [127:0] operand_b,
    input  wire [3:0]   operation,
    output reg  [127:0] result,
    output reg          valid_operation
);
    localparam ALU_ADD = 4'd0;
    localparam ALU_SUB = 4'd1;
    localparam ALU_AND = 4'd2;
    localparam ALU_OR  = 4'd3;
    localparam ALU_XOR = 4'd4;
    localparam ALU_SLL = 4'd5;
    localparam ALU_SRL = 4'd6;

    always @* begin
        result = 128'd0;
        valid_operation = 1'b1;
        case (operation)
            ALU_ADD: result = operand_a + operand_b;
            ALU_SUB: result = operand_a - operand_b;
            ALU_AND: result = operand_a & operand_b;
            ALU_OR:  result = operand_a | operand_b;
            ALU_XOR: result = operand_a ^ operand_b;
            ALU_SLL: result = operand_a << operand_b[6:0];
            ALU_SRL: result = operand_a >> operand_b[6:0];
            default: begin
                result = 128'd0;
                valid_operation = 1'b0;
            end
        endcase
    end
endmodule


module trion_h_regfile (
    input  wire         clk,
    input  wire         reset,
    input  wire         write_enable,
    input  wire [4:0]   write_address,
    input  wire [127:0] write_data,
    input  wire [4:0]   read_address_a,
    input  wire [4:0]   read_address_b,
    output wire [127:0] read_data_a,
    output wire [127:0] read_data_b
);
    reg [127:0] registers [0:31];
    integer index;

    assign read_data_a = (read_address_a == 5'd0) ? 128'd0 : registers[read_address_a];
    assign read_data_b = (read_address_b == 5'd0) ? 128'd0 : registers[read_address_b];

    always @(posedge clk) begin
        if (reset) begin
            for (index = 0; index < 32; index = index + 1)
                registers[index] <= 128'd0;
        end else begin
            if (write_enable && (write_address != 5'd0))
                registers[write_address] <= write_data;
            registers[0] <= 128'd0;
        end
    end
endmodule


module trion_h_core (
    input  wire         clk,
    input  wire         reset,

    output wire         instruction_enable,
    output wire [63:0]  instruction_address,
    input  wire [31:0]  instruction_data,
    input  wire         instruction_error,

    output wire         data_read_enable,
    output wire         data_write_enable,
    output wire [63:0]  data_address,
    output wire [127:0] data_write_data,
    input  wire [127:0] data_read_data,
    input  wire         data_error,

    output reg          halted,
    output reg          fault,
    output reg  [3:0]   fault_code,
    output reg  [63:0]  fault_pc,
    output reg  [31:0]  fault_instruction,
    output reg  [63:0]  pc,
    output reg          instruction_retired,
    output reg  [63:0]  retired_count
);
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

    localparam ALU_ADD = 4'd0;
    localparam ALU_SUB = 4'd1;
    localparam ALU_AND = 4'd2;
    localparam ALU_OR  = 4'd3;
    localparam ALU_XOR = 4'd4;
    localparam ALU_SLL = 4'd5;
    localparam ALU_SRL = 4'd6;

    localparam FAULT_NONE             = 4'd0;
    localparam FAULT_PC_ALIGNMENT     = 4'd1;
    localparam FAULT_INSTRUCTION_RANGE= 4'd2;
    localparam FAULT_ILLEGAL_OPCODE   = 4'd3;
    localparam FAULT_DATA_ALIGNMENT   = 4'd4;
    localparam FAULT_DATA_RANGE       = 4'd5;

    wire [6:0] opcode;
    wire [4:0] rd;
    wire [4:0] rs1;
    wire [4:0] rs2;
    wire [9:0] immediate;
    wire [127:0] immediate_sext;
    wire [127:0] register_a;
    wire [127:0] register_b;

    reg [3:0] alu_operation;
    wire [127:0] alu_result;
    wire alu_valid;

    reg [127:0] writeback_data;
    reg writeback_enable;
    reg recognized_opcode;
    reg branch_taken;
    reg [63:0] effective_address;
    reg [63:0] next_pc;
    reg [3:0] detected_fault_code;
    reg detected_fault;

    wire running;
    wire [63:0] branch_displacement;

    assign running = !reset && !halted && !fault;
    assign instruction_enable = running;
    assign instruction_address = pc;
    assign data_address = effective_address;
    assign data_write_data = register_b;
    assign branch_displacement = {{52{immediate[9]}}, immediate, 2'b00};

    trion_h_decoder decoder (
        .instruction(instruction_data),
        .opcode(opcode),
        .rd(rd),
        .rs1(rs1),
        .rs2(rs2),
        .immediate(immediate),
        .immediate_sext(immediate_sext)
    );

    trion_h_regfile register_file (
        .clk(clk),
        .reset(reset),
        .write_enable(running && !detected_fault && writeback_enable),
        .write_address(rd),
        .write_data(writeback_data),
        .read_address_a(rs1),
        .read_address_b(rs2),
        .read_data_a(register_a),
        .read_data_b(register_b)
    );

    always @* begin
        alu_operation = ALU_ADD;
        case (opcode)
            OP_ADD: alu_operation = ALU_ADD;
            OP_SUB: alu_operation = ALU_SUB;
            OP_AND: alu_operation = ALU_AND;
            OP_OR:  alu_operation = ALU_OR;
            OP_XOR: alu_operation = ALU_XOR;
            OP_SLL: alu_operation = ALU_SLL;
            OP_SRL: alu_operation = ALU_SRL;
            default: alu_operation = ALU_ADD;
        endcase
    end

    trion_h_alu128 alu (
        .operand_a(register_a),
        .operand_b(register_b),
        .operation(alu_operation),
        .result(alu_result),
        .valid_operation(alu_valid)
    );

    always @* begin
        recognized_opcode = 1'b1;
        writeback_enable = 1'b0;
        writeback_data = 128'd0;
        branch_taken = 1'b0;
        effective_address = register_a[63:0] + {{54{immediate[9]}}, immediate};
        next_pc = pc + 64'd4;

        case (opcode)
            OP_ADD, OP_SUB, OP_AND, OP_OR, OP_XOR, OP_SLL, OP_SRL: begin
                writeback_enable = 1'b1;
                writeback_data = alu_result;
            end
            OP_ADDI: begin
                writeback_enable = 1'b1;
                writeback_data = register_a + immediate_sext;
            end
            OP_LD: begin
                writeback_enable = 1'b1;
                writeback_data = data_read_data;
            end
            OP_ST: begin
                writeback_enable = 1'b0;
            end
            OP_BEQ: begin
                branch_taken = (register_a == register_b);
                if (branch_taken)
                    next_pc = pc + branch_displacement;
            end
            OP_BNE: begin
                branch_taken = (register_a != register_b);
                if (branch_taken)
                    next_pc = pc + branch_displacement;
            end
            OP_JAL: begin
                writeback_enable = 1'b1;
                writeback_data = {{64{1'b0}}, (pc + 64'd4)};
                next_pc = pc + branch_displacement;
            end
            OP_HALT: begin
                writeback_enable = 1'b0;
            end
            default: begin
                recognized_opcode = 1'b0;
            end
        endcase

        detected_fault = 1'b0;
        detected_fault_code = FAULT_NONE;

        if (running) begin
            if (pc[1:0] != 2'b00) begin
                detected_fault = 1'b1;
                detected_fault_code = FAULT_PC_ALIGNMENT;
            end else if (instruction_error) begin
                detected_fault = 1'b1;
                detected_fault_code = FAULT_INSTRUCTION_RANGE;
            end else if (!recognized_opcode || !alu_valid &&
                         (opcode == OP_ADD || opcode == OP_SUB ||
                          opcode == OP_AND || opcode == OP_OR ||
                          opcode == OP_XOR || opcode == OP_SLL ||
                          opcode == OP_SRL)) begin
                detected_fault = 1'b1;
                detected_fault_code = FAULT_ILLEGAL_OPCODE;
            end else if ((opcode == OP_LD || opcode == OP_ST) &&
                         (effective_address[3:0] != 4'b0000)) begin
                detected_fault = 1'b1;
                detected_fault_code = FAULT_DATA_ALIGNMENT;
            end else if ((opcode == OP_LD || opcode == OP_ST) && data_error) begin
                detected_fault = 1'b1;
                detected_fault_code = FAULT_DATA_RANGE;
            end
        end
    end

    assign data_read_enable = running && !instruction_error &&
                              (opcode == OP_LD) &&
                              (effective_address[3:0] == 4'b0000);
    assign data_write_enable = running && !instruction_error &&
                               (opcode == OP_ST) &&
                               (effective_address[3:0] == 4'b0000);

    always @(posedge clk) begin
        instruction_retired <= 1'b0;

        if (reset) begin
            pc <= 64'd0;
            halted <= 1'b0;
            fault <= 1'b0;
            fault_code <= FAULT_NONE;
            fault_pc <= 64'd0;
            fault_instruction <= 32'd0;
            retired_count <= 64'd0;
        end else if (running) begin
            if (detected_fault) begin
                fault <= 1'b1;
                fault_code <= detected_fault_code;
                fault_pc <= pc;
                fault_instruction <= instruction_data;
            end else if (opcode == OP_HALT) begin
                halted <= 1'b1;
                instruction_retired <= 1'b1;
                retired_count <= retired_count + 64'd1;
            end else begin
                pc <= next_pc;
                instruction_retired <= 1'b1;
                retired_count <= retired_count + 64'd1;
            end
        end
    end
endmodule


// Simulation memory system. The processor core above has external memory
// interfaces suitable for FPGA block RAM or a system bus. This wrapper gives
// Icarus/Verilator a deterministic, initialized instruction/data memory.
module trion_h_sim_system #(
    parameter integer IMEM_WORDS = 256,
    parameter integer DMEM_WORDS = 256,
    parameter IMEM_INIT_FILE = "",
    parameter DMEM_INIT_FILE = "",
    parameter integer LOAD_DEMO = 0
) (
    input  wire        clk,
    input  wire        reset,
    output wire        halted,
    output wire        fault,
    output wire [3:0]  fault_code,
    output wire [63:0] fault_pc,
    output wire [31:0] fault_instruction,
    output wire [63:0] pc,
    output wire        instruction_retired,
    output wire [63:0] retired_count
);
    reg [31:0] imem [0:IMEM_WORDS-1];
    reg [127:0] dmem [0:DMEM_WORDS-1];

    wire instruction_enable;
    wire [63:0] instruction_address;
    wire [31:0] instruction_data;
    wire instruction_error;
    wire data_read_enable;
    wire data_write_enable;
    wire [63:0] data_address;
    wire [127:0] data_write_data;
    wire [127:0] data_read_data;
    wire data_error;

    integer index;

    function [31:0] encode;
        input [6:0] operation;
        input [4:0] destination;
        input [4:0] source_a;
        input [4:0] source_b;
        input [9:0] imm;
        begin
            encode = {operation, destination, source_a, source_b, imm};
        end
    endfunction

    wire instruction_index_valid;
    assign instruction_index_valid = (instruction_address[63:10] == 0) &&
                                     (instruction_address[9:2] < IMEM_WORDS);
    assign instruction_error = instruction_enable && !instruction_index_valid;
    assign instruction_data = instruction_index_valid ?
        imem[instruction_address[9:2]] : 32'd0;

    wire data_index_valid;
    assign data_index_valid = (data_address[63:12] == 0) &&
                              (data_address[11:4] < DMEM_WORDS);
    assign data_error = (data_read_enable || data_write_enable) &&
                        !data_index_valid;
    assign data_read_data = data_index_valid ?
        dmem[data_address[11:4]] : 128'd0;

    always @(posedge clk) begin
        if (data_write_enable && !data_error)
            dmem[data_address[11:4]] <= data_write_data;
    end

    initial begin
        for (index = 0; index < IMEM_WORDS; index = index + 1)
            imem[index] = encode(7'h7E, 5'd0, 5'd0, 5'd0, 10'd0);
        for (index = 0; index < DMEM_WORDS; index = index + 1)
            dmem[index] = 128'd0;

        if (IMEM_INIT_FILE != "")
            $readmemh(IMEM_INIT_FILE, imem);
        if (DMEM_INIT_FILE != "")
            $readmemh(DMEM_INIT_FILE, dmem);

        if (LOAD_DEMO != 0) begin
            imem[0] = encode(7'h08, 5'd1, 5'd0, 5'd0, 10'd1);
            imem[1] = encode(7'h08, 5'd2, 5'd0, 5'd0, 10'd2);
            imem[2] = encode(7'h01, 5'd3, 5'd1, 5'd2, 10'd0);
            imem[3] = encode(7'h7E, 5'd0, 5'd0, 5'd0, 10'd0);
        end
    end

    trion_h_core cpu (
        .clk(clk),
        .reset(reset),
        .instruction_enable(instruction_enable),
        .instruction_address(instruction_address),
        .instruction_data(instruction_data),
        .instruction_error(instruction_error),
        .data_read_enable(data_read_enable),
        .data_write_enable(data_write_enable),
        .data_address(data_address),
        .data_write_data(data_write_data),
        .data_read_data(data_read_data),
        .data_error(data_error),
        .halted(halted),
        .fault(fault),
        .fault_code(fault_code),
        .fault_pc(fault_pc),
        .fault_instruction(fault_instruction),
        .pc(pc),
        .instruction_retired(instruction_retired),
        .retired_count(retired_count)
    );
endmodule


// Makerchip-compatible top-level wrapper. 'passed' indicates the bundled
// deterministic demo reached HALT without a processor fault; it is not a
// substitute for the dedicated self-checking testbench.
module top (
    input  wire clk,
    input  wire reset_async,
    output wire passed,
    output wire failed
);
    reg reset_sync_a;
    reg reset_sync_b;
    wire halted;
    wire fault;
    wire [3:0] fault_code;
    wire [63:0] fault_pc;
    wire [31:0] fault_instruction;
    wire [63:0] pc;
    wire retired;
    wire [63:0] retired_count;

    always @(posedge clk or posedge reset_async) begin
        if (reset_async) begin
            reset_sync_a <= 1'b1;
            reset_sync_b <= 1'b1;
        end else begin
            reset_sync_a <= 1'b0;
            reset_sync_b <= reset_sync_a;
        end
    end

    trion_h_sim_system #(.LOAD_DEMO(1)) demo_system (
        .clk(clk),
        .reset(reset_sync_b),
        .halted(halted),
        .fault(fault),
        .fault_code(fault_code),
        .fault_pc(fault_pc),
        .fault_instruction(fault_instruction),
        .pc(pc),
        .instruction_retired(retired),
        .retired_count(retired_count)
    );

    assign passed = halted && !fault;
    assign failed = fault;
endmodule
