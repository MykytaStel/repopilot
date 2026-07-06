export function findUser(req: Request) {
  const id = req.query.id;
  return userRepository.findById(id);
}
