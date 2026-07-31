import { Controller, Get, Param, ParseIntPipe } from "@nestjs/common";

@Controller("users")
export class UsersController {
  @Get(":id")
  findOne(@Param("id") id: string) {
    return { id };
  }
}
