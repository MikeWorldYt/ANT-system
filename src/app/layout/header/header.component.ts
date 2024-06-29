import { NgClass } from '@angular/common';
import { Component, OnInit } from '@angular/core';
import { MenuService } from '../../services/menu.service';

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [
    NgClass
  ],
  templateUrl: './header.component.html',
  styleUrl: './header.component.css'
})
export class HeaderComponent implements OnInit{
  menu: boolean = true;
  constructor(private menuService: MenuService) {}

  ngOnInit() {
    this.menuService.currentMenu.subscribe(menu => this.menu = menu);
  }

}
