import { render, screen } from "@testing-library/react";
import { Table, TableFooter, TableHead, TableHeader, TableRow } from "./table";

it("uses the theme border for table dividers", () => {
  render(
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>名称</TableHead>
        </TableRow>
      </TableHeader>
      <TableFooter>
        <TableRow>
          <TableHead>合计</TableHead>
        </TableRow>
      </TableFooter>
    </Table>,
  );

  expect(screen.getByText("名称").closest("tr")).toHaveClass("border-border");
  expect(screen.getByText("合计").closest("tfoot")).toHaveClass("border-border");
});
