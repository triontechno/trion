`timescale 1ns/1ps

// TRION-H Phase 1 consolidated RTL.
// 128-bit datapath, 32 x 128-bit GPRs, 64-bit PC, instruction/data memory,
// branches, jumps, load/store, halt, and packed balanced-ternary arithmetic.
// Instruction: opcode[31:25] | rd[24:20] | rs1[19:15] | rs2[14:10] | imm[9:0].
// Ternary encoding per trit: 00=0, 01=+1, 10=-1, 11=invalid.

module trion_h_decoder (
    input wire [31:0] instruction,
    output wire [6:0] opcode,
    output wire [4:0] rd, rs1, rs2,
    output wire [9:0] immediate
);
    assign opcode=instruction[31:25];
    assign rd=instruction[24:20];
    assign rs1=instruction[19:15];
    assign rs2=instruction[14:10];
    assign immediate=instruction[9:0];
endmodule

module trion_h_core (
    input wire clk, reset,
    output reg halted, fault,
    output reg [63:0] pc,
    output reg [31:0] last_instruction,
    output reg instruction_retired
);
    localparam OP_ADD=7'h01, OP_SUB=7'h02, OP_AND=7'h03,
      OP_OR=7'h04, OP_XOR=7'h05, OP_SLL=7'h06, OP_SRL=7'h07,
      OP_ADDI=7'h08, OP_LD=7'h09, OP_ST=7'h0A, OP_BEQ=7'h0B,
      OP_BNE=7'h0C, OP_JAL=7'h0D, OP_TADD=7'h10,
      OP_TSUB=7'h11, OP_HALT=7'h7E;

    localparam ALU_ADD=4'd0, ALU_SUB=4'd1, ALU_AND=4'd2,
      ALU_OR=4'd3, ALU_XOR=4'd4, ALU_SLL=4'd5, ALU_SRL=4'd6,
      ALU_TADD=4'd7, ALU_TSUB=4'd8;

    reg [31:0] imem[0:255];
    reg [127:0] dmem[0:255];
    reg [127:0] gpr[0:31];

    wire [31:0] instruction;
    wire [6:0] opcode;
    wire [4:0] rd,rs1,rs2;
    wire [9:0] imm10;
    reg [3:0] alu_operation;
    reg [127:0] alu_a,alu_b,alu_result;
    reg alu_valid;
    reg [127:0] writeback_value;
    reg writeback_enable;
    reg [63:0] next_pc,effective_address;
    reg recognized;
    integer k;

    assign instruction=imem[pc[9:2]];
    trion_h_decoder decoder(
      .instruction(instruction),.opcode(opcode),.rd(rd),
      .rs1(rs1),.rs2(rs2),.immediate(imm10)
    );

    always @* begin
      alu_a=(rs1==0)?128'd0:gpr[rs1];
      alu_b=(rs2==0)?128'd0:gpr[rs2];
      alu_operation=ALU_ADD;
      case(opcode)
        OP_ADD:alu_operation=ALU_ADD;
        OP_SUB:alu_operation=ALU_SUB;
        OP_AND:alu_operation=ALU_AND;
        OP_OR:alu_operation=ALU_OR;
        OP_XOR:alu_operation=ALU_XOR;
        OP_SLL:alu_operation=ALU_SLL;
        OP_SRL:alu_operation=ALU_SRL;
        OP_TADD:alu_operation=ALU_TADD;
        OP_TSUB:alu_operation=ALU_TSUB;
        default:alu_operation=ALU_ADD;
      endcase
      if(opcode==OP_ADDI || opcode==OP_LD || opcode==OP_ST)
        alu_b={{118{imm10[9]}},imm10};
    end

    reg [127:0] ternary_alu_result;
    reg ternary_alu_valid;
    integer ti,ta,tb,td,tc;

    always @* begin
      ternary_alu_result=128'd0;
      ternary_alu_valid=1'b1;
      tc=0;
      for(ti=0;ti<64;ti=ti+1) begin
        case(alu_a[ti*2 +: 2])
          2'b00:ta=0;
          2'b01:ta=1;
          2'b10:ta=-1;
          default:begin ta=0;ternary_alu_valid=1'b0;end
        endcase
        case(alu_b[ti*2 +: 2])
          2'b00:tb=0;
          2'b01:tb=1;
          2'b10:tb=-1;
          default:begin tb=0;ternary_alu_valid=1'b0;end
        endcase
        if(alu_operation==ALU_TSUB) tb=-tb;
        td=ta+tb+tc;
        tc=0;
        if(td>1)begin td=td-3;tc=1;end
        else if(td < -1)begin td=td+3;tc=-1;end
        case(td)
          0:ternary_alu_result[ti*2 +: 2]=2'b00;
          1:ternary_alu_result[ti*2 +: 2]=2'b01;
          -1:ternary_alu_result[ti*2 +: 2]=2'b10;
          default:begin
            ternary_alu_result[ti*2 +: 2]=2'b00;
            ternary_alu_valid=1'b0;
          end
        endcase
      end
    end

    always @* begin
      alu_result=128'd0;
      alu_valid=1'b1;
      case(alu_operation)
        ALU_ADD:alu_result=alu_a+alu_b;
        ALU_SUB:alu_result=alu_a-alu_b;
        ALU_AND:alu_result=alu_a&alu_b;
        ALU_OR:alu_result=alu_a|alu_b;
        ALU_XOR:alu_result=alu_a^alu_b;
        ALU_SLL:alu_result=alu_a<<alu_b[6:0];
        ALU_SRL:alu_result=alu_a>>alu_b[6:0];
        ALU_TADD,ALU_TSUB:begin
          alu_result=ternary_alu_result;
          alu_valid=ternary_alu_valid;
        end
        default:begin alu_result=0;alu_valid=0;end
      endcase
    end

    always @* begin
      recognized=1'b1;
      writeback_enable=1'b0;
      writeback_value=alu_result;
      effective_address=alu_a[63:0]+{{54{imm10[9]}},imm10};
      next_pc=pc+64'd4;
      case(opcode)
        OP_ADD,OP_SUB,OP_AND,OP_OR,OP_XOR,OP_SLL,OP_SRL,
        OP_TADD,OP_TSUB:begin
          writeback_enable=1'b1;
          writeback_value=alu_result;
        end
        OP_ADDI:begin
          writeback_enable=1'b1;
          writeback_value=alu_a+{{118{imm10[9]}},imm10};
        end
        OP_LD:begin
          writeback_enable=1'b1;
          writeback_value=dmem[effective_address[11:4]];
        end
        OP_ST:writeback_enable=1'b0;
        OP_BEQ:if(alu_a==alu_b)
          next_pc=pc+($signed({{54{imm10[9]}},imm10}) <<< 2);
        OP_BNE:if(alu_a!=alu_b)
          next_pc=pc+($signed({{54{imm10[9]}},imm10}) <<< 2);
        OP_JAL:begin
          writeback_enable=1'b1;
          writeback_value={{64{1'b0}},(pc+64'd4)};
          next_pc=pc+($signed({{54{imm10[9]}},imm10}) <<< 2);
        end
        OP_HALT:writeback_enable=1'b0;
        default:recognized=1'b0;
      endcase
      if((opcode==OP_TADD || opcode==OP_TSUB) && !alu_valid)
        recognized=1'b0;
      if(opcode==OP_LD && effective_address[3:0]!=0)recognized=1'b0;
      if(opcode==OP_ST && effective_address[3:0]!=0)recognized=1'b0;
    end

    always @(posedge clk) begin
      instruction_retired<=1'b0;
      if(reset)begin
        pc<=64'd0;
        halted<=1'b0;
        fault<=1'b0;
        last_instruction<=32'd0;
        for(k=0;k<32;k=k+1)gpr[k]<=128'd0;
      end else if(!halted && !fault)begin
        last_instruction<=instruction;
        if(!recognized)fault<=1'b1;
        else if(opcode==OP_HALT)begin
          halted<=1'b1;
          instruction_retired<=1'b1;
        end else begin
          if(opcode==OP_ST)
            dmem[effective_address[11:4]]<=(rs2==0)?128'd0:gpr[rs2];
          if(writeback_enable && rd!=0)
            gpr[rd]<=writeback_value;
          pc<=next_pc;
          instruction_retired<=1'b1;
        end
        gpr[0]<=128'd0;
      end
    end
endmodule

module trion_h_alu128(
    input wire [127:0] operand_a,operand_b,
    input wire [3:0] operation,
    output wire [127:0] result,
    output wire valid_operation
);
    wire [127:0] r;
    wire v;
    trion_h_alu128_impl impl(
      .a(operand_a),.b(operand_b),.operation(operation),
      .result(r),.valid(v)
    );
    assign result=r;
    assign valid_operation=v;
endmodule

module trion_h_alu128_impl(
    input wire [127:0] a,b,
    input wire [3:0] operation,
    output reg [127:0] result,
    output reg valid
);
    integer n,x,y,c,d;
    reg ok;
    always @* begin
      result=0;valid=1;c=0;ok=1;
      case(operation)
        0:result=a+b;
        1:result=a-b;
        2:result=a&b;
        3:result=a|b;
        4:result=a^b;
        5:result=a<<b[6:0];
        6:result=a>>b[6:0];
        7,8:begin
          for(n=0;n<64;n=n+1)begin
            case(a[n*2 +: 2])
              0:x=0;1:x=1;2:x=-1;
              default:begin x=0;ok=0;end
            endcase
            case(b[n*2 +: 2])
              0:y=0;1:y=1;2:y=-1;
              default:begin y=0;ok=0;end
            endcase
            if(operation==8)y=-y;
            d=x+y+c;c=0;
            if(d>1)begin d=d-3;c=1;end
            else if(d < -1)begin d=d+3;c=-1;end
            case(d)
              0:result[n*2 +: 2]=0;
              1:result[n*2 +: 2]=1;
              -1:result[n*2 +: 2]=2;
              default:begin result[n*2 +: 2]=0;ok=0;end
            endcase
          end
          valid=ok;
        end
        default:begin result=0;valid=0;end
      endcase
    end
endmodule
