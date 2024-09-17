import { Component } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import { HeaderComponent } from '../../layout/header/header.component';
import { SidebarComponent } from '../../layout/sidebar/sidebar.component';
import { MainComponent } from '../../layout/main/main.component';
import { NgClass } from '@angular/common';
import { MenuService } from '../../services/menu.service';
import { SummaryBarComponent } from '../../layout/summary-bar/summary-bar.component';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [
      NgClass,
      RouterOutlet,
      HeaderComponent,
      SidebarComponent,
      MainComponent,
      SummaryBarComponent
    ],
  templateUrl: './home.component.html',
  styleUrl: './home.component.css'
})
export default class HomeComponent {
  // Menu
    menu: boolean;
    constructor(private menuService: MenuService) {
      this.menu = window.innerWidth > 768 ? true : false;
    }

  showMenu() {
    this.menu = !this.menu;
    this.menuService.changeMenu(this.menu);
  }

  closeMenu(event: MouseEvent) {
    // Verifica si el ancho de la pantalla es menor a 768px
    if (window.innerWidth >= 768) {
      return;
    }
    // cierra el menu, si se hace click fuera (dispo moviles)
    const target = event.target as HTMLElement;
    const aside = document.querySelector('aside.menu-container');
    if (this.menu && aside && !aside.contains(target)) {
      this.menu = false;
      this.menuService.changeMenu(this.menu);
    }
  }
}