import { Controller, Get, Param } from "@nestjs/common";

@Controller("users")
export class UsersController {
  @Get(":id")
  findOne() {
    const id = "system";
    return { id };
  }
}
