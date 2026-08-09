import { Controller, Get, Param } from "@nestjs/common";

class CustomPipe {}

@Controller("users")
export class UsersController {
  @Get(":id")
  findOne(@Param("id", CustomPipe) id: string) {
    return db.query("SELECT * FROM users WHERE id = $1", [id]);
  }
}
